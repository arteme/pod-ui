use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic;
use std::time::Duration;
use hibitset::{BitSet, BitSetLike, DrainableBitSet};
use log::{debug, error, warn};
use Origin::{MIDI, UI};
use pod_core::context::Ctx;
use pod_core::controller::*;
use pod_core::cc_values::*;
use pod_core::event::*;
use pod_core::generic;
use pod_core::generic::num_program;
use pod_core::handler::Handler;
use pod_core::midi::MidiMessage;
use pod_core::model::{AbstractControl, Config};
use pod_core::names::ProgramNames;
use crate::tuner::Tuner;

/// A marker to send MidiMessage::XtPatchDumpEnd
const MARKER_PATCH_DUMP_END: u32 = 0x0001;
/// A marker that store status hasn't been received (timed out)
const MARKER_STORE_STATUS_TIMEOUT: u32 = 0x0002;
/// A marker that program number & edit status should be requested
const MARKER_REQUEST_PROGRAM_NUMBER: u32 = 0x0003;

struct Inner {
    midi_out_queue: VecDeque<MidiMessage>,
    /// Send XtStoreStatus ack message when the XtPatchDump message is
    /// received (from Line6 Edit)
    need_store_ack: bool,
    /// Programs that were sent as `03 71` messages that need to be ack'ed
    /// with an XtStoreStatus message
    store_programs: BitSet,
    /// A JoinHandle for the currently running thread waiting for the
    /// timeout of the XtStoreStatus message
    store_status_timeout_handler: Option<tokio::task::JoinHandle<()>>,
    tuner: Option<Tuner>,
    /// Effects
    effects: ProgramNames,
    /// The latest received XtProgramNumber
    reported_program_number: Option<usize>
}

pub(crate) struct PodXtHandler {
    inner: RefCell<Inner>
}

impl PodXtHandler {
    pub fn new(config: &Config) -> Self {
        let inner = Inner {
            midi_out_queue: VecDeque::new(),
            need_store_ack: false,
            store_programs: BitSet::with_capacity(128),
            store_status_timeout_handler: None,
            tuner: None,
            effects: ProgramNames::new_with_size(config, 64),
            reported_program_number: None,
        };
        Self { inner: RefCell::new(inner) }
    }

    pub fn queue_send(&self, ctx: &Ctx) -> bool {
        let inner = self.inner.borrow_mut();
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

    pub fn need_store_ack(&self) -> bool {
        self.inner.borrow().need_store_ack
    }

    pub fn set_need_store_ack(&self, value: bool) {
        self.inner.borrow_mut().need_store_ack = value
    }

    fn tuner_on(&self, ctx: &Ctx, origin: Origin) {
        let mut inner = self.inner.borrow_mut();
        if inner.tuner.is_none() {
            let mut tuner = Tuner::new();
            tuner.start(ctx);
            inner.tuner.replace(tuner);

            ctx.controller.set("tuner_enable", 1, origin.into());
            ctx.set_program(Program::Tuner, origin);
        }
    }

    fn tuner_off(&self, ctx: &Ctx, origin: Origin) {
        let mut inner = self.inner.borrow_mut();
        inner.tuner.take(); // take & drop tuner

        ctx.controller.set("tuner_enable", 0, origin.into());
    }

    fn tuner_toggle(&self, on: bool, ctx: &Ctx, origin: Origin) {
        if on {
            self.tuner_on(ctx, origin)
        } else {
            self.tuner_off(ctx, origin)
        }
    }
}

impl Handler for PodXtHandler {
    fn cc_handler(&self, ctx: &Ctx, event: &ControlChangeEvent) {
        if event.name.as_str() == "tuner_enable" && event.origin == StoreOrigin::MIDI {
            let on = event.value > 0;
            self.tuner_toggle(on, ctx, MIDI);
            // when MIDI device turns off the tuner, ask it, what program is on now
            if !on {
                let msg = MidiMessage::XtProgramNumberRequest;
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
            }
        }

        if event.name.as_str() == "effect_select" && event.origin == StoreOrigin::MIDI {
            let inner = self.inner.borrow();
            let name = inner.effects.get(event.value as usize).unwrap_or_default();
            let num = event.value + 1;

            let msg = if !name.is_empty() {
                format!("Effect select: {} - {}", num, name)
            } else {
                format!("Effect select: {}", num)
            };
            let e = NotificationEvent { msg, id: Some("effect_select".into()) };
            ctx.app_event_tx.send_or_warn(AppEvent::Notification(e));
        }

        generic::cc_handler(ctx, event);

    }

    fn pc_handler(&self, ctx: &Ctx, event: &ProgramChangeEvent) {
        self.tuner_toggle(event.program == Program::Tuner, ctx, event.origin);

        generic::pc_handler(ctx, event);
    }

    fn load_handler(&self, ctx: &Ctx, event: &BufferLoadEvent) {
        match event.origin {
            MIDI => {
                generic::load_handler(ctx, event);
                // Send a marker that an XtPatchDumpEnd is needed to be sent.
                ctx.app_event_tx.send_or_warn(AppEvent::Marker(MARKER_PATCH_DUMP_END));
            }
            UI => {
                match event.buffer {
                    Buffer::EditBuffer => {
                        let msg = MidiMessage::XtEditBufferDumpRequest;
                        ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
                    }
                    Buffer::Current => {
                        if let Some(v) = num_program(&ctx.program()) {
                            self.queue_push(
                                MidiMessage::XtPatchDumpRequest { patch: v as u16 }
                            );
                            self.queue_send(ctx);
                        }
                    }
                    Buffer::Program(v) => {
                        self.queue_push(MidiMessage::XtPatchDumpRequest { patch: v as u16 });
                        self.queue_send(ctx);
                    }
                    Buffer::All => {
                        // Request patches
                        for v in 0 .. ctx.config.program_num {
                            self.queue_push(MidiMessage::XtPatchDumpRequest { patch: v as u16 });
                        }
                        // Request effects
                        for v in 0 .. 64 {
                            self.queue_push(MidiMessage::XtPatchDumpRequest { patch: 0x0200 | v as u16 });
                        }
                        self.queue_send(ctx);
                    }
                };
            }
        }
    }

    fn store_handler(&self, ctx: &Ctx, event: &BufferStoreEvent) {
        if self.inner.borrow().store_status_timeout_handler.is_none() {
            generic::store_handler(ctx, event);
            // The generic handler sends 1..N buffer dump messages.
            // Send a marker that an XtPatchDumpEnd is needed to be sent.
            ctx.app_event_tx.send_or_warn(AppEvent::Marker(MARKER_PATCH_DUMP_END));
        }
    }

    fn buffer_handler(&self, ctx: &Ctx, event: &BufferDataEvent) {
        match event.origin {
            MIDI => {
                generic::buffer_handler(ctx, event);
                generic::buffer_modified_handler(ctx, event);
                if event.request == MIDI {
                    // patch dump `03 71` messages need to be acknowledged
                    self.set_need_store_ack(true)
                }

                // Collect effects names
                match event.buffer {
                    Buffer::Program(p) if p >= 0x0200 => {
                        let num = p & 0xff;

                        let mut inner = self.inner.borrow_mut();
                        inner.effects.update_from_data(num, event.data.as_slice(), StoreOrigin::MIDI);
                    }
                    _ => {}
                }
            }
            UI => {
                if event.request == UI {
                    error!("Store events from UI not implemented")
                }
                let patch = match event.buffer {
                    Buffer::Current | Buffer::All => {
                        // Buffer::Current is already converted into Buffer::Program(_)
                        // by the store handler!
                        // Buffer::All is split into single buffers by store handler!
                        error!("Unsupported event: {:?}", event);
                        return;
                    }
                    Buffer::EditBuffer => {
                        // edit buffer dump is always sent as a buffer dump
                        None
                    }
                    Buffer::Program(_) if event.request == MIDI => {
                        // request from MIDI is answered with a buffer dump
                        None
                    }
                    Buffer::Program(p) /*if event.request == Origin::UI*/ => {
                        // this is a user action, send a patch dump
                        Some(p)
                    }
                };

                let msg = if let Some(patch) = patch {
                    // record that store (patch dump) was senta for patch
                    self.inner.borrow_mut().store_programs.add(patch as u32);
                    // send a patch dump
                    MidiMessage::XtPatchDump {
                        patch: patch as u16,
                        id: ctx.config.member as u8,
                        data: event.data.clone()
                    }
                } else {
                    // send a buffer dump
                    MidiMessage::XtBufferDump {
                        id: ctx.config.member as u8,
                        data: event.data.clone()
                    }
                };
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
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
                ctx.controller.set("xt_packs", *packs as u16, MIDI.into());
            }

            MidiMessage::XtEditBufferDumpRequest => {
                let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: MIDI };
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
                        warn!("Can't determine incoming buffer designation, queue peek = {:?}", msg);
                        // the origin of this buffer dump is likely a "save" button
                        // pressed on the device, store the dump to the edit buffer
                        Buffer::EditBuffer
                    }
                };
                // PODxt buffer dump `03 74` is a reply for an edit buffer dump
                // request `03 75` or a patch dump request `03 73`, so the request
                // origin is "UI"
                let e = BufferDataEvent {
                    buffer,
                    origin: MIDI,
                    request: UI,
                    data: data.clone()
                };
                ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
            }
            MidiMessage::XtPatchDumpRequest { patch } => {
                let e = BufferLoadEvent { buffer: Buffer::Program(*patch as usize), origin: MIDI };
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
                // PODxt patch dump `03 71` originates is sent by the device
                // or Line6 Edit, so the request origin is "MIDI"
                let e = BufferDataEvent {
                    buffer: Buffer::Program(*patch as usize),
                    origin: MIDI,
                    request: MIDI,
                    data: data.clone()
                };
                ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
            }
            MidiMessage::XtPatchDumpEnd => {
                if self.need_store_ack() {
                    // send store status message as ack message
                    let msg = MidiMessage::XtStoreStatus { success: true };
                    ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
                } else {
                    // send next message
                    self.queue_pop();
                    self.queue_send(ctx);
                }
            }
            MidiMessage::XtStoreStatus { success } => {
                let mut inner = self.inner.borrow_mut();
                inner.store_status_timeout_handler.take()
                    .map(|h| h.abort());

                if *success {
                    for patch in inner.store_programs.drain() {
                        let e = ModifiedEvent {
                            buffer: Buffer::Program(patch as usize),
                            origin: MIDI,
                            modified: false
                        };
                        ctx.app_event_tx.send_or_warn(AppEvent::Modified(e));
                    }
                } else {
                    let msg = format!("Storing programs to the device failed!");
                    error!("{}", msg);
                    let e = NotificationEvent { msg, id: None };
                    ctx.app_event_tx.send_or_warn(AppEvent::Notification(e));

                    inner.store_programs.clear();
                }
            }
            MidiMessage::XtTunerNoteRequest => {
                // when Line6 Edit asks, animate the tuner indicator
                let (note, _) = tuner_value_next(0);
                let msg = MidiMessage::XtTunerNote { note };
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
            }
            MidiMessage::XtTunerOffsetRequest => {
                // when Line6 Edit asks, animate the tuner indicator
                let (_, offset) = tuner_value_next(3);
                let msg = MidiMessage::XtTunerOffset { offset };
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
            }
            MidiMessage::XtTunerNote { note } => {
                ctx.controller.set("tuner_note", *note, MIDI.into());
            }
            MidiMessage::XtTunerOffset { offset } => {
                ctx.controller.set("tuner_offset", *offset, MIDI.into());
            }
            MidiMessage::XtProgramNumberRequest => {
                if let Some(program) = num_program(&ctx.program()) {
                    let msg = MidiMessage::XtProgramNumber { program: program as u16 };
                    ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
                }
            }
            MidiMessage::XtProgramNumber { program } => {
                ctx.set_program(Program::Program(*program), MIDI);
                self.inner.borrow_mut().reported_program_number = Some(*program as usize);

                // If this was a reply to an XtProgramNumberRequest, advance the message queue
                match self.queue_peek() {
                    Some(MidiMessage::XtProgramNumberRequest) => {
                        self.queue_pop();
                        self.queue_send(ctx);
                    }
                    _ => {}
                }
            }
            MidiMessage::XtProgramEditStateRequest => {
                if let Some(program) = num_program(&ctx.program()) {
                    let edited = ctx.dump.lock().unwrap().modified(program);
                    let msg = MidiMessage::XtProgramEditState { edited };
                    ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
                }
            }
            MidiMessage::XtProgramEditState { edited } => {
                if let Some(p) = self.inner.borrow().reported_program_number {
                    let e = ModifiedEvent {
                        buffer: Buffer::Program(p),
                        origin: MIDI,
                        modified: *edited
                    };
                    ctx.app_event_tx.send_or_warn(AppEvent::Modified(e));

                    if ctx.program() == Program::Program(p as u16) {
                        // same program selected in the app as in the device,
                        // request the edit buffer dump
                        let e = BufferLoadEvent {
                            buffer: Buffer::EditBuffer,
                            origin: UI
                        };
                        ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
                    }
                }

                // If this was a reply to an XtProgramEditStateRequest, advance the message queue
                match self.queue_peek() {
                    Some(MidiMessage::XtProgramEditStateRequest) => {
                        self.queue_pop();
                        self.queue_send(ctx);
                    }
                    _ => {}
                }
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

        // request selected program number & edit status,
        // but defer this to the end of the queue by sending a marker
        ctx.app_event_tx.send_or_warn(AppEvent::Marker(MARKER_REQUEST_PROGRAM_NUMBER));
    }

    fn marker_handler(&self, ctx: &Ctx, marker: u32) {
        match marker {
            MARKER_PATCH_DUMP_END => {
                let msg = MidiMessage::XtPatchDumpEnd;
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));

                if !self.inner.borrow().store_programs.is_empty() {
                    let handler = tokio::spawn({
                        let app_event_tx = ctx.app_event_tx.clone();
                        async move {
                            tokio::time::sleep(Duration::from_millis(5000)).await;
                            app_event_tx.send_or_warn(AppEvent::Marker(MARKER_STORE_STATUS_TIMEOUT));
                            ()
                        }
                    });
                    self.inner.borrow_mut().store_status_timeout_handler.replace(handler);
                }
            }
            MARKER_STORE_STATUS_TIMEOUT => {
                // We've not received a store status message, empty the programs bitset
                let mut inner = self.inner.borrow_mut();
                inner.store_programs.clear();
                inner.store_status_timeout_handler.take();
                // TODO: show error in UI?
            }
            MARKER_REQUEST_PROGRAM_NUMBER => {
                // Queue program number request and edit status request messages
                let msg = MidiMessage::XtProgramNumberRequest;
                self.queue_push(msg);
                let msg = MidiMessage::XtProgramEditStateRequest;
                self.queue_push(msg);
            }
            _ => {
                error!("Marker {} not handled!", marker);
            }
        }
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
        let control_value = control.value_from_midi(value);

        controller.set_cc_value(cc, value, StoreOrigin::NONE);
        controller.set(name, control_value, StoreOrigin::NONE);
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

fn tuner_value_next(inc: u16) -> (u16, u16) {
    static TUNER_VALUE: atomic::AtomicU16 = atomic::AtomicU16::new(0);
    let v = TUNER_VALUE.fetch_add(inc, atomic::Ordering::SeqCst);

    let offset = ((v % 100) as i16 - 50) as u16;
    let note = (v / 100) % 12;

    (note, offset)
}