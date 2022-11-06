use log::{error, warn};
use crate::config::{GUI, MIDI};
use crate::context::Ctx;
use crate::controller::*;
use crate::dump::ProgramsDump;
use crate::event::{AppEvent, ControlChangeEvent, EventSender, EventSenderExt, Origin, Program, ProgramChangeEvent};
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
    let updated = update_control(ctx, event);
    if updated {
        update_dump(ctx, event);
        send_midi_cc(ctx, event);
    }
}

pub fn midi_cc_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ControlChange { channel, control, value } = midi_message else {
        warn!("Incorrect MIDI message for MIDI CC handler: {:?}", midi_message);
        return;
    };

    let expected_channel = ctx.midi_channel();
    if expected_channel != Channel::all() && *channel != expected_channel {
        // Ignore midi messages sent to a different channel
        return;
    }



}




pub fn pc_handler(ctx: &Ctx, event: &ProgramChangeEvent) {

}