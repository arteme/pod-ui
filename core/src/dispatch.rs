use std::collections::HashMap;
use std::sync::Mutex;
use log::*;
use once_cell::sync::Lazy;
use crate::context::Ctx;
use crate::event::*;
use crate::midi::{Channel, MidiMessage};

/// DISPATCH_BUFFER_REROUTE is a hash map of Buffer -> Buffer routing,
/// used when an unmodified program (load from device) is requested into
/// the edit buffer (possibly, into a different program) from a program
/// that is not the current program.
static DISPATCH_BUFFER_REROUTE: Lazy<Mutex<HashMap<Buffer, Buffer>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

pub fn dispatch_buffer_set(from: Buffer, to: Buffer) {
    DISPATCH_BUFFER_REROUTE.lock().unwrap().insert(from, to);
}

pub fn dispatch_buffer_get(from: &Buffer) -> Option<Buffer> {
    DISPATCH_BUFFER_REROUTE.lock().unwrap().remove(from)
}

pub fn dispatch_buffer_clear() {
    DISPATCH_BUFFER_REROUTE.lock().unwrap().clear()
}

// -------------------------------------------------------------

pub fn cc_handler(ctx: &Ctx, event: &ControlChangeEvent) {
    ctx.handler.cc_handler(ctx, event)
}

pub fn midi_cc_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ControlChange { channel, .. } = midi_message else {
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
    let MidiMessage::ControlChange { .. } = midi_message else {
        warn!("Incorrect MIDI message for MIDI CC handler: {:?}", midi_message);
        return;
    };

    ctx.handler.midi_cc_out_handler(ctx, midi_message);
}

pub fn midi_pc_in_handler(ctx: &Ctx, midi_message: &MidiMessage) {
    let MidiMessage::ProgramChange { channel, .. } = midi_message else {
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
    let MidiMessage::ProgramChange { .. } = midi_message else {
        warn!("Incorrect MIDI message for MIDI PC handler: {:?}", midi_message);
        return;
    };

    ctx.handler.midi_pc_out_handler(ctx, midi_message);
}

pub fn pc_handler(ctx: &Ctx, event: &ProgramChangeEvent) {
    dispatch_buffer_clear();
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
    match dispatch_buffer_get(&event.buffer) {
        Some(buffer) => {
            info!("Dispatch buffer rerouted: {:?} -> {:?}", event.buffer, buffer);
            // reroute buffer event to a new buffer
            let mut event = event.clone();
            event.buffer = buffer;
            ctx.handler.buffer_handler(ctx, &event)
        }
        None => {
            // process buffer data event as-is
            ctx.handler.buffer_handler(ctx, event)
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
    if channel != Channel::all() && channel != expected_channel {
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

pub fn marker_handler(ctx: &Ctx, marker: u32) {
    ctx.handler.marker_handler(ctx, marker)
}