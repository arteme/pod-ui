use log::{error, warn};
use crate::config::{GUI, MIDI};
use crate::context::Ctx;
use crate::controller::*;
use crate::dump::ProgramsDump;
use crate::event::{AppEvent, Buffer, BufferLoadEvent, ControlChangeEvent, EventSender, EventSenderExt, ModifiedEvent, Origin, Program, ProgramChangeEvent};
use crate::midi::{Channel, MidiMessage};
use crate::model::{AbstractControl, Config, Control};
use crate::stack::ControllerStack;

fn update_control(ctx: &Ctx, event: &ControlChangeEvent) -> bool {
    let origin = match event.origin {
        Origin::UI => GUI,
        Origin::MIDI => MIDI,
    };

    ctx.controller.set(event.name.as_str(), event.value, origin)
}

fn update_dump(ctx: &Ctx, event: &ControlChangeEvent) {
    let controller = &ctx.controller.lock().unwrap();
    let dump = &mut ctx.dump.lock().unwrap();

    let program = controller.get("program").unwrap();
    let program = Program::from(program);
    let idx = match program {
        Program::ManualMode => {
            todo!()
        }
        Program::Tuner => {
            todo!()
        }
        Program::Program(v) => { v as usize }
    };

    let Some(buffer) = dump.data_mut(idx) else {
        warn!("No dump buffer for program {}", idx);
        return;
    };

    control_value_to_buffer(controller, event, buffer);
}

fn control_value_to_buffer(controller: &Controller, event: &ControlChangeEvent, buffer: &mut [u8]) {
    todo!()
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
            let updated = update_control(ctx, event);
            if updated {
                update_dump(ctx, event);
            }
        }
        Origin::UI => {
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
    let modified = controller.set(name, value, MIDI);
    if modified {
        // CC from MIDI -> set modified flag
        // todo: make sure this gets handled
        let e = ModifiedEvent { buffer: Buffer::Current, modified: true, origin: Origin::MIDI };
        ctx.app_event_tx.send_or_warn(AppEvent::Modified(e));
    }
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

    // todo
}

pub fn midi_pc_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ProgramChange { channel, program } = midi_message else {
        warn!("Incorrect MIDI message for MIDI PC handler: {:?}", midi_message);
        return;
    };

    // todo
}

pub fn pc_handler(ctx: &Ctx, event: &ProgramChangeEvent) {
}

// other

pub fn midi_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
}

pub fn midi_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
}


