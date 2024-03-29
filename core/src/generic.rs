use log::{error, warn};
use crate::context::Ctx;
use crate::controller::*;
use crate::event::*;
use crate::event::Origin::{MIDI, UI};
use crate::midi::{Channel, MidiMessage};
use crate::model::{AbstractControl, DeviceFlags};
use crate::{config, program};
use crate::cc_values::*;
use crate::dispatch::dispatch_buffer_set;

fn update_edit_buffer(ctx: &Ctx, event: &ControlChangeEvent) {
    let controller = &ctx.controller.lock().unwrap();
    let edit = ctx.edit.lock().unwrap();
    let mut raw = edit.raw_locked();
    ctx.handler.control_value_to_buffer(controller, event.name.as_str(), &mut raw);
}

fn update_current_program_name(ctx: &Ctx) -> bool {
    let Some(program) = num_program(&ctx.program()) else {
        return false;
    };

    let name = ctx.edit.lock().unwrap().name();
    ctx.dump.lock().unwrap().set_name(program, name, UI)
}

fn send_midi_cc(ctx: &Ctx, event: &ControlChangeEvent) {
    let ControlChangeEvent { name, value, origin } = event;
    if *origin != StoreOrigin::UI {
        return;
    }

    let Some(control) = &ctx.controller.get_config(name) else {
        warn!("Control {:?} not found!", name);
        return;
    };

    let Some(cc) = control.get_cc() else {
        // skip virtual controls
        return;
    };

    let channel = ctx.midi_channel();
    let channel = if channel == Channel::all() { 0 } else { channel };

    let value = control.value_to_midi(*value);
    let msg = MidiMessage::ControlChange { channel, control: cc, value };
    ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
}

pub fn cc_handler(ctx: &Ctx, event: &ControlChangeEvent) {
    // Only controls that have an address in the buffer should trigger
    // a "buffer modified" event. Also name change triggers "buffer modified"
    let has_addr = ctx.controller.get_config(&event.name)
        .and_then(|c| c.get_addr()).is_some();
    let is_name_change = event.name == "name_change";
    let modified = match event.origin {
        StoreOrigin::MIDI => {
            update_edit_buffer(ctx, event);
            has_addr
        }
        _ if is_name_change => {
            update_current_program_name(ctx)
        }
        StoreOrigin::UI => {
            update_edit_buffer(ctx, event);
            send_midi_cc(ctx, event);
            has_addr
        }
        _ => false
    };
    if modified {
        let e = ModifiedEvent {
            buffer: Buffer::Current,
            // event.origin may be StoreOrigin::None if this is a name change,
            // override it with MIDI in that case
            origin: Origin::try_from(event.origin).unwrap_or(MIDI),
            modified: true,
        };
        ctx.app_event_tx.send_or_warn(AppEvent::Modified(e));
    }
}

pub fn midi_cc_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ControlChange { control: cc, value, .. } = midi_message else {
        warn!("Incorrect MIDI message for MIDI CC handler: {:?}", midi_message);
        return;
    };

    let config = ctx.config;
    if config.in_cc_edit_buffer_dump_req.contains(cc) {
        // send an "edit buffer dump request"
        let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: UI };
        ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
    }

    let Some((name, control)) = ctx.config.cc_to_control(*cc) else {
        warn!("Control for CC={} not defined!", cc);
        return;
    };

    let mut controller = ctx.controller.lock().unwrap();

    // save raw CC value to the controller
    controller.set_cc_value(*cc, *value, MIDI.into());

    // save converted value to the controller
    let value = control.value_from_midi(*value);
    controller.set(name, value, MIDI.into());
}

pub fn midi_cc_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ControlChange { control: cc, value, .. } = midi_message else {
        warn!("Incorrect MIDI message for MIDI CC handler: {:?}", midi_message);
        return;
    };

    let config = ctx.config;
    if config.out_cc_edit_buffer_dump_req.contains(cc) {
        // send an "edit buffer dump request"
        let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: UI };
        ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
    }

    // save raw CC value to the controller
    ctx.controller.lock().unwrap().set_cc_value(*cc, *value, UI.into());
}

pub fn midi_pc_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ProgramChange { program, .. } = midi_message else {
        warn!("Incorrect MIDI message for MIDI PC handler: {:?}", midi_message);
        return;
    };

    let program_range = 0 .. ctx.config.program_num;
    let program = match *program as usize {
        p if ctx.config.pc_manual_mode.is_some_and(|v| v == p) => Program::ManualMode,
        p if ctx.config.pc_tuner.is_some_and(|v| v == p) => Program::Tuner,
        p => {
            let offset = ctx.config.pc_offset.unwrap_or_default();
            let p = p - offset;
            if program_range.contains(&p) {
                Program::Program(p as u16)
            } else {
                error!("Incorrect program in PC message: {}", p);
                return;
            }
        }
    };


    ctx.set_program(program, MIDI);
}

pub fn midi_pc_out_handler(_ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ProgramChange { .. } = midi_message else {
        warn!("Incorrect MIDI message for MIDI PC handler: {:?}", midi_message);
        return;
    };
}

pub fn pc_handler(ctx: &Ctx, event: &ProgramChangeEvent) {
    let modified = sync_edit_and_dump_buffers(ctx, event.origin);

    if event.origin == UI {
        let program = match event.program {
            Program::ManualMode => ctx.config.pc_manual_mode,
            Program::Tuner => ctx.config.pc_tuner,
            // TODO: fix magic 1000
            Program::Program(1000) => None,
            Program::Program(v) => {
                let offset = ctx.config.pc_offset.unwrap_or_default();
                Some(v as usize + offset)
            }
        };
        if let Some(program) = program {
            let msg = MidiMessage::ProgramChange { channel: ctx.midi_channel(), program: program as u8 };
            if modified && ctx.config.flags.contains(DeviceFlags::MODIFIED_BUFFER_PC_AND_EDIT_BUFFER) {
                // The buffer is modified, so the  send_edit_buffer_or_pc will send
                // the edit buffer. Send PC here first if the device can handle it.
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg.clone()));
            }

            send_edit_buffer_or_pc(ctx, modified, msg);
        }
    }
}

pub fn sync_edit_and_dump_buffers(ctx: &Ctx, origin: Origin) -> bool {
    let mut edit = ctx.edit.lock().unwrap();
    let mut dump = ctx.dump.lock().unwrap();

    let prev_program = num_program(&ctx.program_prev());
    if let Some(page) = prev_program {
        // store edit buffer to the program dump
        let data = program::store_patch_dump_ctrl(&edit);
        program::load_patch_dump(&mut dump, page, data.as_slice(), origin);
        // in case the edit buffer was modified, but the dump was not marked
        // modified (as happens with a name change signal), make sure to
        // send a "modified event"
        if edit.modified() {
            dump.set_modified(page, true);
            let e = ModifiedEvent {
                buffer: Buffer::Program(page),
                origin,
                modified: true
            };
            ctx.app_event_tx.send_or_warn(AppEvent::Modified(e));
        }
    }

    let mut modified = false;
    let program = num_program(&ctx.program());
    if let Some(page) = program {
        // load data from product dump to edit buffer
        let data = dump.data(page).unwrap();

        // In case of program change, always send a signal that the data change is coming
        // from MIDI so that the GUI gets updated, but the MIDI does not
        let value_fn = |controller: &mut Controller, name: &str, buffer: &[u8]|
            ctx.handler.control_value_from_buffer(controller, name, buffer);
        program::load_patch_dump_ctrl(&mut edit, data, value_fn);
        modified = dump.modified(page);
        edit.set_modified(modified);
    }

    ctx.set_program_prev(ctx.program(), origin);

    modified
}


pub fn send_edit_buffer_or_pc(ctx: &Ctx, modified: bool, pc: MidiMessage) {
    if modified {
        // send edit buffer
        let e = BufferStoreEvent {
            buffer: Buffer::EditBuffer,
            origin: UI
        };
        ctx.app_event_tx.send_or_warn(AppEvent::Store(e));
    } else {
        // send PC
        ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(pc));
    };
}

// other

#[allow(unused_variables)]
pub fn midi_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
}

#[allow(unused_variables)]
pub fn midi_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
}

// load & store

pub fn load_handler(ctx: &Ctx, event: &BufferLoadEvent) {
    match event.origin {
        MIDI => {
            // reroute this to the store handler
            let e = BufferStoreEvent { buffer: event.buffer.clone(), origin: MIDI };
            store_handler(ctx, &e);
        }
        UI => {
            let msg = match event.buffer {
                Buffer::EditBuffer => {
                    Some(MidiMessage::ProgramEditBufferDumpRequest)
                }
                Buffer::Current => {
                    let patch = num_program(&ctx.program());
                    patch.map(|v| {
                        MidiMessage::ProgramPatchDumpRequest { patch: v as u8 }
                    })
                }
                Buffer::Program(v) => {
                    Some(MidiMessage::ProgramPatchDumpRequest { patch: v as u8 })
                }
                Buffer::All => {
                    Some(MidiMessage::AllProgramsDumpRequest)
                }
            };
            if let Some(msg) = msg {
                ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg))
            }
        }
    }
}

pub fn store_handler(ctx: &Ctx, event: &BufferStoreEvent) -> bool {
    // Store request origin
    let request = event.origin;
    let origin = UI;

    let dump = ctx.dump.lock().unwrap();
    match event.buffer {
        Buffer::EditBuffer => {
            let e = BufferDataEvent {
                request,
                origin,
                buffer: Buffer::EditBuffer,
                data: program::store_patch_dump_ctrl(&ctx.edit.lock().unwrap())
            };
            ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
        }
        Buffer::Current => {
            let patch = num_program(&ctx.program());
            let Some(patch) = patch else {
                warn!("Program {:?}, current buffer store discarded", ctx.program());
                return false;
            };
            let e = BufferDataEvent {
                request,
                origin,
                buffer: Buffer::Program(patch),
                data: program::store_patch_dump(&dump, patch)
            };
            ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
        }
        Buffer::Program(patch) => {
            let e = BufferDataEvent {
                request,
                origin,
                buffer: Buffer::Program(patch),
                data: program::store_patch_dump(&dump, patch)
            };
            ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
        }
        Buffer::All => {
            if ctx.config.flags.contains(DeviceFlags::ALL_PROGRAMS_DUMP) {
                // all programs in a single dump message
                let e = BufferDataEvent {
                    request,
                    origin,
                    buffer: Buffer::All,
                    data: program::store_all_dump(&dump)
                };
                ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
            } else {
                // individual program dump messages for each program
                for patch in 0 .. ctx.config.program_num {
                    let e = BufferDataEvent {
                        request,
                        origin,
                        buffer: Buffer::Program(patch),
                        data: program::store_patch_dump(&dump, patch)
                    };
                    ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
                }
            }
        }
    }
    true
}

pub fn copy_handler(ctx: &Ctx, event: &BufferCopyEvent) {
    dispatch_buffer_set(event.from.clone(), event.to.clone());
    let e = BufferStoreEvent {
        buffer: event.from.clone(),
        origin: UI
    };
    ctx.app_event_tx.send_or_warn(AppEvent::Store(e));
}

pub fn buffer_handler(ctx: &Ctx, event: &BufferDataEvent) {
    let value_fn = |controller: &mut Controller, name: &str, buffer: &[u8]| {
        ctx.handler.control_value_from_buffer(controller, name, buffer)
    };
    match event.origin {
        MIDI => {
            let update_edit_buffer = match event.buffer {
                Buffer::EditBuffer => {
                    program::load_patch_dump_ctrl(
                        &mut ctx.edit.lock().unwrap(),
                        event.data.as_slice(),
                        value_fn
                    );
                    false
                }
                Buffer::Current => {
                    // MIDI send "current" buffer dumps, only numbered ones
                    error!("Unsupported event: {:?}", event);
                    return;
                }
                Buffer::Program(program) => {
                    program::load_patch_dump(
                        &mut ctx.dump.lock().unwrap(),
                        program,
                        event.data.as_slice(),
                        MIDI
                    );
                    // if the program with the same index is selected, then also
                    // update the edit buffer
                    ctx.program() == Program::Program(program as u16)
                }
                Buffer::All => {
                    program::load_all_dump(
                        &mut ctx.dump.lock().unwrap(),
                        event.data.as_slice(),
                        MIDI
                    );
                    // update edit buffer
                    true
                }
            };
            if update_edit_buffer {
                let Some(current) = num_program(&ctx.program()) else {
                    warn!("Update edit buffer flag, but program: {:?}", ctx.program());
                    return;
                };
                program::load_patch_dump_ctrl(
                    &mut ctx.edit.lock().unwrap(),
                    ctx.dump.lock().unwrap().data(current).unwrap(),
                    value_fn
                );
            }
        }
        UI => {
            let msg = match event.buffer {
                Buffer::EditBuffer => {
                    MidiMessage::ProgramEditBufferDump { ver: 0, data: event.data.clone() }
                }
                Buffer::Current => {
                    // Buffer::Current is already converted into Buffer::Program(_)
                    // by the store handler!
                    error!("Unsupported event: {:?}", event);
                    return;
                }
                Buffer::Program(v) => {
                    MidiMessage::ProgramPatchDump {
                        patch: v as u8,
                        ver: 0,
                        data: event.data.clone()
                    }
                }
                Buffer::All => {
                    MidiMessage::AllProgramsDump {
                        ver: 0,
                        data: event.data.clone()
                    }
                }
            };
            ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
        }
    }
}

/// A generic handler that converts BufferData events into Modified events
pub fn buffer_modified_handler(ctx: &Ctx, event: &BufferDataEvent, rerouted: bool) {
    match event.buffer {
        Buffer::EditBuffer => {
            // buffer data event directed to the edit buffer should always set
            // the `modified` flag because it is either one of:
            // - an edit buffer load from incoming CC event (knob switched on the device)
            // - an edit buffer load from outgoing CC event (select modified in the UI)
            // - a "load this program into the edit buffer" event, which modifies the
            //   current edit buffer
            let e = ModifiedEvent {
                buffer: event.buffer.clone(),
                origin: event.origin,
                modified: true,
            };
            ctx.app_event_tx.send_or_warn(AppEvent::Modified(e));
            // if the event is a rerouted buffer event (#3 cause as described above)
            // make sure the edit buffer is also sent to the device
            if rerouted {
                let e = BufferStoreEvent {
                    buffer: Buffer::EditBuffer,
                    origin: UI
                };
                ctx.app_event_tx.send_or_warn(AppEvent::Store(e));
            }
        }
        Buffer::Current => {
            // devices do not send "current" buffer dumps, only numbered ones
            error!("Unsupported event: {:?}", event);
            return;
        }
        Buffer::Program(_) | Buffer::All => {
            // buffer data event directed to a specific buffer should always clear
            // the `modified` unless it's a reroute (store from edit buffer or
            // other program via UI request)
            let e = ModifiedEvent {
                buffer: event.buffer.clone(),
                origin: event.origin,
                modified: rerouted,
            };
            ctx.app_event_tx.send_or_warn(AppEvent::Modified(e));
        }
    }
}

pub fn midi_udi_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    match midi_message {
        MidiMessage::UniversalDeviceInquiry { channel } => {
            // Pretend we're the POD model that is currently loaded
            let msg = MidiMessage::UniversalDeviceInquiryResponse {
                channel: *channel,
                family: ctx.config.family,
                member: ctx.config.member,
                ver: "0303".to_string()
            };
            ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
        }
        MidiMessage::UniversalDeviceInquiryResponse { family, member, ver, .. } => {
            let c1 = &ver.chars().next().unwrap_or_default();
            let version = if ('0' ..= '9').contains(c1) {
                let hi = if *c1 == '0' { &ver[1 ..= 1] } else { &ver[0 ..= 1] };
                let lo = &ver[2 ..= 3];
                format!("{}.{}", hi, lo)
            } else {
                let mut bytes = ver.bytes();
                let b1 = bytes.next().unwrap_or_default();
                let b2 = bytes.next().unwrap_or_default();
                let b3 = bytes.next().unwrap_or_default();
                let b4 = bytes.next().unwrap_or_default();
                if b1 == 0 && b3 == 0 {
                    format!("{}.{:02}", b2, b4)
                } else {
                    error!("Unsupported version string: {:?}", midi_message);
                    "?.?".to_string()
                }
            };

            let name = config::config_for_id(*family, *member)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("Unknown ({:04x}:{:04x})", family, member));

            let e = DeviceDetectedEvent { name, version };
            ctx.app_event_tx.send_or_warn(AppEvent::DeviceDetected(e));
        }
        _ => {}
    }
}


pub fn modified_handler(ctx: &Ctx, event: &ModifiedEvent) {
    let mut dump = ctx.dump.lock().unwrap();
    match event.buffer {
        Buffer::EditBuffer => {
            ctx.edit.lock().unwrap().set_modified(event.modified);
        }
        Buffer::Current => {
            let program = num_program(&ctx.program());
            if let Some(p) = program {
                dump.set_modified(p, event.modified);
                ctx.edit.lock().unwrap().set_modified(event.modified);
            }
        }
        Buffer::Program(p) => {
            dump.set_modified(p, event.modified);
        }
        Buffer::All => {
            dump.set_all_modified(event.modified);
        }
    }
}

pub fn new_device_handler(ctx: &Ctx) {
    // Request device id
    let msg = MidiMessage::UniversalDeviceInquiry { channel: ctx.midi_channel() };
    ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));

    // Request all buffers load
    let e = BufferLoadEvent { buffer: Buffer::All, origin: UI };
    ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
}


/// Convert `Program` to an `Option` of a number if a program is
/// a number program and not a manual mode or tuner
pub fn num_program(p: &Program) -> Option<usize> {
    match p {
        Program::ManualMode | Program::Tuner => { None }
        Program::Program(1000) => { None }
        Program::Program(v) => { Some(*v as usize) }
    }
}