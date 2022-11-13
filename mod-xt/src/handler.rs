use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
use pod_core::context::Ctx;
use pod_core::event::*;
use pod_core::generic;
use pod_core::generic::num_program;
use pod_core::handler::Handler;
use pod_core::midi::MidiMessage;

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
            MidiMessage::XtPatchDumpEnd => {
                // send next message
                self.queue_pop();
                self.queue_send(ctx);
            }
            _ => {}
        }
    }

    fn new_device_handler(&self, ctx: &Ctx) {
        generic::new_device_handler(ctx);

        // detect installed packs
        let msg = MidiMessage::XtInstalledPacksRequest;
        ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
    }


}