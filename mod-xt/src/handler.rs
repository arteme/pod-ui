use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
use log::{debug, error, warn};
use pod_core::config::MIDI;
use pod_core::context::Ctx;
use pod_core::controller::*;
use pod_core::cc_values::*;
use pod_core::event::*;
use pod_core::{controller, generic};
use pod_core::generic::num_program;
use pod_core::handler::Handler;
use pod_core::midi::MidiMessage;
use pod_core::model::AbstractControl;

struct Inner {
    midi_out_queue: VecDeque<MidiMessage>,
    sent: bool
}

pub(crate) struct PodXtHandler {
    inner: RefCell<Inner>
}

impl PodXtHandler {
    pub fn new() -> Self {
        let inner = Inner {
            midi_out_queue: VecDeque::new(),
            sent: false
        };
        Self { inner: RefCell::new(inner) }
    }

    pub fn queue_send(&self, ctx: &Ctx) -> bool {
        let mut inner = self.inner.borrow_mut();
        if let Some(msg) = inner.midi_out_queue.front() {
            ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg.clone()));
            true
        } else {
            false
        }
    }

    pub fn queue_push(&self, message: MidiMessage) {
        self.inner.borrow_mut().midi_out_queue.push_back(message);
    }

    pub fn queue_pop(&self) {
        self.inner.borrow_mut().midi_out_queue.pop_front();
    }

    pub fn queue_peek(&self) -> Option<MidiMessage> {
        self.inner.borrow_mut().midi_out_queue.get(0).cloned()
    }
}

impl Handler for PodXtHandler {
    fn load_handler(&self, ctx: &Ctx, event: &BufferLoadEvent) {
        match event.origin {
            Origin::MIDI => {
                generic::load_handler(ctx, event)
            }
            Origin::UI => {
                match event.buffer {
                    Buffer::EditBuffer => {
                        self.queue_push(MidiMessage::XtEditBufferDumpRequest);
                    }
                    Buffer::Current => {
                        let patch = num_program(&ctx.program());
                        patch.map(|v| {
                            self.queue_push(
                                MidiMessage::XtPatchDumpRequest { patch: v as u16 }
                            )
                        });
                    }
                    Buffer::Program(v) => {
                        self.queue_push(MidiMessage::XtPatchDumpRequest { patch: v as u16 });
                    }
                    Buffer::All => {
                        for v in 0 .. ctx.config.program_num {
                            self.queue_push(MidiMessage::XtPatchDumpRequest { patch: v as u16 });
                        }
                    }
                };
                self.queue_send(ctx);
            }
        }
    }

    fn midi_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_in_handler(ctx, midi_message);

        match midi_message {
            MidiMessage::XtInstalledPacksRequest => {
                // when Line6 Edit asks, we report we have all packs
                let msg = MidiMessage::XtInstalledPacks { packs: 0x0f };
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
            }
            MidiMessage::XtInstalledPacks { packs } => {
                ctx.controller.set("xt_packs", *packs as u16, MIDI);
            }

            MidiMessage::XtEditBufferDumpRequest => {
                let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: Origin::MIDI };
                ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
            }
            MidiMessage::XtBufferDump { id, data } => {
                if *id != (ctx.config.member as u8) {
                    warn!("Buffer dump id mismatch: expected {}, got {}", ctx.config.member, id);
                }
                if data.len() != ctx.config.program_size {
                    error!("Program size mismatch: expected {}, got {}",
                       ctx.config.program_size, data.len());
                    return;
                }
                // PODxt answers with a buffer dump to either edit buffer dump request or
                // a patch dump request... We peek into the current queue to try and determine,
                // which buffer comes. This is quite error-prone, since any one message missed
                // may incorrectly place the data into the wrong dump ;(
                let buffer = match self.queue_peek() {
                    Some(MidiMessage::XtEditBufferDumpRequest) =>
                        Buffer::EditBuffer,
                    Some(MidiMessage::XtPatchDumpRequest { patch }) =>
                        Buffer::Program(patch as usize),
                    msg @ _ => {
                        error!("Can't determine incoming buffer designation, queue peek = {:?}", msg);
                        return;
                    }
                };

                let e = BufferDataEvent {
                    buffer,
                    origin: Origin::MIDI,
                    data: data.clone()
                };
                ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
            }
            MidiMessage::XtPatchDumpRequest { patch } => {
                let e = BufferLoadEvent { buffer: Buffer::Program(*patch as usize), origin: Origin::MIDI };
                ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
            }
            MidiMessage::XtPatchDump { patch, id, data } => {
                if *id != (ctx.config.member as u8) {
                    warn!("Buffer dump id mismatch: expected {}, got {}", ctx.config.member, id);
                }
                if data.len() != ctx.config.program_size {
                    error!("Program size mismatch: expected {}, got {}",
                       ctx.config.program_size, data.len());
                    return;
                }
                let e = BufferDataEvent {
                    buffer: Buffer::Program(*patch as usize),
                    origin: Origin::MIDI,
                    data: data.clone()
                };
                ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
            }

            MidiMessage::XtPatchDumpEnd => {
                // send next message
                self.queue_pop();
                self.queue_send(ctx);
            }
            // TODO: handle XtSaved
            _ => {}
        }
    }

    fn new_device_handler(&self, ctx: &Ctx) {
        generic::new_device_handler(ctx);

        // detect installed packs
        let msg = MidiMessage::XtInstalledPacksRequest;
        ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
    }

    fn control_value_from_buffer(&self, controller: &mut Controller, name: &str, buffer: &[u8]) {
        let Some(control) = controller.get_config(name) else {
            return;
        };
        let Some(cc) = control.get_cc() else {
            // skip virtual controls
            return;
        };
        let addr = match control.get_addr() {
            Some((addr, len)) if len == 1 => addr,
            Some((_, len)) => {
                error!("PODxt control_value_to_buffer: len={} for control {:?} not supported!", len, name);
                return;
            }
            None => {
                debug!("MIDI-only control {:?} skipped", name);
                return;
            }
        };

        let value = buffer[addr as usize];
        let control_value = controller.get(name).unwrap();
        let value = control.value_from_midi(value, control_value);
        controller.set(name, value, MIDI);
    }


    // PODxt writes raw MIDI data to the buffer
    fn control_value_to_buffer(&self, controller: &Controller, name: &str, buffer: &mut [u8]) {
        let Some(control) = controller.get_config(name) else {
            return;
        };
        let Some(cc) = control.get_cc() else {
            // skip virtual controls
            return;
        };
        let addr = match control.get_addr() {
            Some((addr, len)) if len == 1 => addr,
            Some((_, len)) => {
                error!("PODxt control_value_to_buffer: len={} for control {:?} not supported!", len, name);
                return;
            }
            None => {
                debug!("MIDI-only control {:?} skipped", name);
                return;
            }
        };

        let Some(value) = controller.get_cc_value(cc) else {
            error!("No raw CC value for CC={}", cc);
            return;
        };
        buffer[addr as usize] = value;
    }
}