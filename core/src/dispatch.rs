use log::*;
use crate::context::Ctx;
use crate::event::*;
use crate::midi::{Channel, MidiMessage};

pub fn cc_handler(ctx: &Ctx, event: &ControlChangeEvent) {
    ctx.handler.cc_handler(ctx, event)
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

    ctx.handler.midi_cc_in_handler(ctx, midi_message)
}

pub fn midi_cc_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ControlChange { channel, control: cc, value } = midi_message else {
        warn!("Incorrect MIDI message for MIDI CC handler: {:?}", midi_message);
        return;
    };

    ctx.handler.midi_cc_out_handler(ctx, midi_message);
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

    ctx.handler.midi_pc_in_handler(ctx, midi_message);
}

pub fn midi_pc_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ProgramChange { channel, program } = midi_message else {
        warn!("Incorrect MIDI message for MIDI PC handler: {:?}", midi_message);
        return;
    };

    ctx.handler.midi_pc_out_handler(ctx, midi_message);
}

pub fn pc_handler(ctx: &Ctx, event: &ProgramChangeEvent) {
    ctx.handler.pc_handler(ctx, event);
}

// other

pub fn midi_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    ctx.handler.midi_in_handler(ctx, midi_message)
}

pub fn midi_out_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    ctx.handler.midi_out_handler(ctx, midi_message)
}

// load & store

pub fn load_handler(ctx: &Ctx, event: &BufferLoadEvent) {
    ctx.handler.load_handler(ctx, event)
}

pub fn store_handler(ctx: &Ctx, event: &BufferStoreEvent) {
    match event.origin {
        Origin::MIDI => {
            // This should never happen!
            error!("Unsupported event: {:?}", event);
            return;
        }
        _ => {}
    }

    ctx.handler.store_handler(ctx, event)
}

pub fn buffer_handler(ctx: &Ctx, event: &BufferDataEvent) {
    ctx.handler.buffer_handler(ctx, event)
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

    ctx.handler.midi_udi_handler(ctx, midi_message)
}


pub fn modified_handler(ctx: &Ctx, event: &ModifiedEvent) {
    ctx.handler.modified_handler(ctx, event)
}

pub fn new_device_handler(ctx: &Ctx) {
    ctx.handler.new_device_handler(ctx)
}
