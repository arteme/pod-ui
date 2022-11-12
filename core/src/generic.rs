use log::{error, warn};
use crate::config::{GUI, MIDI};
use crate::context::Ctx;
use crate::controller::*;
use crate::event::{AppEvent, Buffer, BufferDataEvent, BufferLoadEvent, BufferStoreEvent, ControlChangeEvent, DeviceDetectedEvent, EventSender, EventSenderExt, ModifiedEvent, Origin, Program, ProgramChangeEvent};
use crate::midi::{Channel, MidiMessage};
use crate::model::{AbstractControl, Control, DeviceFlags};
use crate::{config, program};

fn update_dump(ctx: &Ctx, event: &ControlChangeEvent) {
    let controller = &ctx.controller.lock().unwrap();
    let dump = &mut ctx.dump.lock().unwrap();

    let Some(idx) = num_program(ctx.program()) else {
        // not updating dump in manual mode or tuner
        return;
    };

    let Some(buffer) = dump.data_mut(idx) else {
        warn!("No dump buffer for program {}", idx);
        return;
    };

    control_value_to_buffer(controller, event, buffer);
}

fn control_value_to_buffer(controller: &Controller, event: &ControlChangeEvent, buffer: &mut [u8]) {
    // todo!()
}

fn send_midi_cc(ctx: &Ctx, event: &ControlChangeEvent) {
    let ControlChangeEvent { name, value, origin } = event;
    if *origin != Origin::UI {
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

    let config = ctx.config;

    // TODO: rewrite this after changing multi-byte controls to MSB,LSB pair...
    // Map the control address to a list of controls to make CC messages for.
    // Typically this will be a single-element list with the same control
    // that was resolved by name. For multibyte controls this will be a list
    // of [tail control, head control], sorted this way specifically because
    // we want to sent the lower bytes first.
    let controls: Vec<&Control> = control.get_addr()
        .filter(|(_, size)| *size > 1)
        .map(|(addr, size)| {
            // multibyte control
            config.addr_to_control_vec((addr + size -1) as usize, true)
                .into_iter().map(|(_, c)| c).collect()
        })
        // single byte control, or control without address
        .unwrap_or_else(|| vec![control]);

     let channel = ctx.midi_channel();
    let channel = if channel == Channel::all() { 0 } else { channel };

    let messages = controls.into_iter().map(|control| {
        let value = control.value_to_midi(*value);
        let cc = control.get_cc();
        MidiMessage::ControlChange { channel, control: cc.unwrap(), value }
    }).collect::<Vec<_>>();

    // send messages
    for msg in messages.into_iter() {
        ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
    }
}

pub fn cc_handler(ctx: &Ctx, event: &ControlChangeEvent) {
    match event.origin {
        Origin::MIDI => {
            update_dump(ctx, event);
        }
        Origin::UI => {
            update_dump(ctx, event);
            send_midi_cc(ctx, event);
        }
    }
}

pub fn midi_cc_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ControlChange { channel, control: cc, value } = midi_message else {
        warn!("Incorrect MIDI message for MIDI CC handler: {:?}", midi_message);
        return;
    };

    let expected_channel = ctx.midi_channel();
    if expected_channel != Channel::all() && *channel != expected_channel {
        // Ignore midi messages sent to a different channel
        return;
    }

    let config = ctx.config;
    if config.in_cc_edit_buffer_dump_req.contains(cc) {
        // send an "edit buffer dump request"
        let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: Origin::UI };
        ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
    }

    let Some((name, control)) = ctx.config.cc_to_control(*cc) else {
        warn!("Control for CC={} not defined!", cc);
        return;
    };

    // Map the control address to a control for the value lookup.
    // For most controls this will the same control as the one
    // resolved by CC, for multibyte controls this will be the
    // head control.
    let (name, value_control) = control.get_addr()
        .and_then(|(addr, _)| {
            config.addr_to_control_vec(addr as usize, false).into_iter().next()
        })
        .filter(|(name, control)| {
            let (_, size) = control.get_addr().unwrap();
            size > 1
        })
        // single byte control, or control without address
        .unwrap_or_else(|| (name, control));

    let mut controller = ctx.controller.lock().unwrap();
    let control_value = controller.get(name).unwrap();
    let value = control.value_from_midi(*value, control_value);
    controller.set(name, value, MIDI);
}

pub fn midi_cc_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ControlChange { channel, control: cc, value } = midi_message else {
        warn!("Incorrect MIDI message for MIDI CC handler: {:?}", midi_message);
        return;
    };

    let config = ctx.config;
    if config.out_cc_edit_buffer_dump_req.contains(cc) {
        // send an "edit buffer dump request"
        let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: Origin::UI };
        ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
    }
}

pub fn midi_pc_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ProgramChange { channel, program } = midi_message else {
        warn!("Incorrect MIDI message for MIDI PC handler: {:?}", midi_message);
        return;
    };

    let expected_channel = ctx.midi_channel();
    if expected_channel != Channel::all() && *channel != expected_channel {
        // Ignore midi messages sent to a different channel
        return;
    }

    ctx.set_program(Program::from(*program as u16), MIDI);
}

pub fn midi_pc_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ProgramChange { channel, program } = midi_message else {
        warn!("Incorrect MIDI message for MIDI PC handler: {:?}", midi_message);
        return;
    };

    // todo
}

pub fn pc_handler(ctx: &Ctx, event: &ProgramChangeEvent) {
    match event.origin {
        Origin::MIDI => {
            sync_edit_and_dump_buffers(ctx, MIDI);
        }
        Origin::UI => {
            let modified = sync_edit_and_dump_buffers(ctx, GUI);
            send_midi_pc(ctx, &event.program, modified);
        }
    }

}

pub fn sync_edit_and_dump_buffers(ctx: &Ctx, origin: u8) -> bool {
    let mut edit = ctx.edit.lock().unwrap();
    let mut dump = ctx.dump.lock().unwrap();

    let prev_program = num_program(ctx.program_prev());
    if let Some(page) = prev_program {
        // store edit buffer to the program dump
        let data = program::store_patch_dump_ctrl(&edit);
        program::load_patch_dump(&mut dump, page, data.as_slice(), origin);
        dump.set_modified(page, edit.modified()); // not needed?
    }

    let mut modified = false;
    let program = num_program(ctx.program());
    if let Some(page) = program {
        // load data from product dump to edit buffer
        let data = dump.data(page).unwrap();

        // In case of program change, always send a signal that the data change is coming
        // from MIDI so that the GUI gets updated, but the MIDI does not
        program::load_patch_dump_ctrl(&mut edit, data, MIDI);
        modified = dump.modified(page);
        edit.set_modified(modified);
    }

    ctx.set_program_prev(ctx.program(), origin);

    modified
}


pub fn send_midi_pc(ctx: &Ctx, program: &Program, modified: bool) {
    let send_pc = if modified {
        // send edit buffer
        let e = BufferStoreEvent {
            buffer: Buffer::EditBuffer,
            origin: Origin::UI
        };
        ctx.app_event_tx.send_or_warn(AppEvent::Store(e));
    } else {
        // send PC

        // todo
        let program = match program {
            Program::ManualMode => { None }
            Program::Tuner => { None }
            Program::Program(p) => { Some(*p as u8) }
        };
        if let Some(program) = program {
            let msg = MidiMessage::ProgramChange { channel: ctx.midi_channel(), program };
            ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
        }
    };

    // todo?

}

// other

pub fn midi_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    match midi_message {
        MidiMessage::ProgramPatchDumpRequest { patch } => {
            // TODO: 1-indexed?
            let e = BufferLoadEvent { buffer: Buffer::Program(*patch as usize), origin: Origin::MIDI };
            ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
        }
        MidiMessage::ProgramPatchDump { patch, ver, data } => {
            if *ver != 0 {
                error!("Unsupported patch dump version: {}", ver);
                return;
            }
            if data.len() != ctx.config.program_size {
                error!("Program size mismatch: expected {}, got {}",
                       ctx.config.program_size, data.len());
                return;
            }
            // TODO: 1-indexed?
            let e = BufferDataEvent {
                buffer: Buffer::Program(*patch as usize),
                origin: Origin::MIDI,
                data: data.clone()
            };
            ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
        }
        MidiMessage::ProgramEditBufferDumpRequest => {
            let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: Origin::MIDI };
            ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
        }
        MidiMessage::ProgramEditBufferDump { ver, data } => {
            if *ver != 0 {
                error!("Unsupported patch dump version: {}", ver);
                return;
            }
            if data.len() != ctx.config.program_size {
                error!("Program size mismatch: expected {}, got {}",
                       ctx.config.program_size, data.len());
                return;
            }
            let e = BufferDataEvent {
                buffer: Buffer::EditBuffer,
                origin: Origin::MIDI,
                data: data.clone()
            };
            ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
        }
        MidiMessage::AllProgramsDumpRequest => {
            let e = BufferLoadEvent { buffer: Buffer::All, origin: Origin::MIDI };
            ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
        }
        MidiMessage::AllProgramsDump { ver, data } => {
            if *ver != 0 {
                error!("Unsupported patch dump version: {}", ver);
                return;
            }
            if data.len() != ctx.config.program_size {
                error!("Program size mismatch: expected {}, got {}",
                       ctx.config.program_size, data.len());
                return;
            }
            let e = BufferDataEvent {
                buffer: Buffer::All,
                origin: Origin::MIDI,
                data: data.clone()
            };
            ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));

        }
        // TODO: PODxt messages
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
        MidiMessage::XtEditBufferDump { id, data } => {
            if *id != (ctx.config.member as u8) {
                warn!("Buffer dump id mismatch: expected {}, got {}", ctx.config.member, id);
            }
            if data.len() != ctx.config.program_size {
                error!("Program size mismatch: expected {}, got {}",
                       ctx.config.program_size, data.len());
                return;
            }
            let e = BufferDataEvent {
                buffer: Buffer::EditBuffer,
                origin: Origin::MIDI,
                data: data.clone()
            };
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
        }
        MidiMessage::XtPatchDumpEnd => {
            // TODO!
        }
        _ => {}
    }
}

pub fn midi_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
}

// load & store

pub fn load_handler(ctx: &Ctx, event: &BufferLoadEvent) {
    match event.origin {
        Origin::MIDI => {
            // reroute this to the store handler
            let e = BufferStoreEvent { buffer: event.buffer.clone(), origin: Origin::UI };
            store_handler(ctx, &e);
        }
        Origin::UI => {
            let msg = match event.buffer {
                Buffer::EditBuffer => {
                    Some(MidiMessage::ProgramEditBufferDumpRequest)
                }
                Buffer::Current => {
                    let patch = num_program(ctx.program());
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

pub fn store_handler(ctx: &Ctx, event: &BufferStoreEvent) {
    match event.origin {
        Origin::MIDI => {
            // This should never happen!
            error!("Unsupported event: {:?}", event);
            return;
        }
        Origin::UI => {
            let dump = ctx.dump.lock().unwrap();
            match event.buffer {
                Buffer::EditBuffer => {
                    let e = BufferDataEvent {
                        buffer: Buffer::EditBuffer,
                        origin: Origin::UI,
                        data: program::store_patch_dump_ctrl(&ctx.edit.lock().unwrap())
                    };
                    ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
                }
                Buffer::Current => {
                    let patch = num_program(ctx.program());
                    let Some(patch) = patch else { return };
                    let e = BufferDataEvent {
                        buffer: Buffer::Program(patch),
                        origin: Origin::UI,
                        data: program::store_patch_dump(&dump, patch)
                    };
                    ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
                }
                Buffer::Program(patch) => {
                    let e = BufferDataEvent {
                        buffer: Buffer::Program(patch),
                        origin: Origin::UI,
                        data: program::store_patch_dump(&dump, patch)
                    };
                    ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
                }
                Buffer::All => {
                    if ctx.config.flags.contains(DeviceFlags::ALL_PROGRAMS_DUMP) {
                        // all programs in a single dump message
                        let e = BufferDataEvent {
                            buffer: Buffer::All,
                            origin: Origin::UI,
                            data: program::store_all_dump(&dump)
                        };
                        ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
                    } else {
                        // individual program dump messages for each program
                        for patch in 0 .. ctx.config.program_num {
                            let e = BufferDataEvent {
                                buffer: Buffer::Program(patch),
                                origin: Origin::UI,
                                data: program::store_patch_dump(&dump, patch)
                            };
                            ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
                        }
                    }
                }
            }
        }
    }
}

pub fn buffer_handler(ctx: &Ctx, event: &BufferDataEvent) {
    match event.origin {
        Origin::MIDI => {
            let update_edit_buffer = match event.buffer {
                Buffer::EditBuffer => {
                    program::load_patch_dump_ctrl(
                        &mut ctx.edit.lock().unwrap(),
                        event.data.as_slice(),
                        MIDI
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
                let current = match ctx.program() {
                    Program::Program(v) => { v as usize }
                    _ => {
                        error!("Update edit buffer flag, but program: {:?}", ctx.program());
                        return;
                    }
                };
                program::load_patch_dump_ctrl(
                    &mut ctx.edit.lock().unwrap(),
                    ctx.dump.lock().unwrap().data(current).unwrap(),
                    MIDI
                );
            }
        }
        Origin::UI => {
            // TODO: modified flag
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

pub fn midi_udi_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let channel = match midi_message {
        MidiMessage::UniversalDeviceInquiry { channel } => *channel,
        MidiMessage::UniversalDeviceInquiryResponse { channel, .. } => *channel,
        _ => {
            error!("Incorrect MIDI message for MIDI UDI handler: {:?}", midi_message);
            return;
        }
    };

    let expected_channel = ctx.midi_channel();
    if expected_channel != Channel::all() && channel != expected_channel {
        // Ignore midi messages sent to a different channel
        return;
    }

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
        MidiMessage::UniversalDeviceInquiryResponse { channel, family, member, ver } => {
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
            let program = num_program(ctx.program());
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

    // TODO: This will go to module-specific `new_device_ping`
    match ctx.config.family {
        0x0003 => {
            // PODxt family
            let msg = MidiMessage::XtInstalledPacksRequest;
            ctx.app_event_tx.send_or_warn(AppEvent::MidiMsgOut(msg));
        }
        _ => {
            let e = BufferLoadEvent { buffer: Buffer::All, origin: Origin::UI };
            ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
        }
    }


}


/// Convert `Program` to an `Option` of a number if a program is
/// a number program and not a manual mode or tuner
pub fn num_program(p: Program) -> Option<usize> {
    match p {
        Program::ManualMode | Program::Tuner => { None }
        Program::Program(v) => { Some(v as usize) }
    }
}