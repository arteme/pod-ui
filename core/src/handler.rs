#![allow(unused_variables)]
use crate::context::Ctx;
use crate::controller::Controller;
use crate::event::*;
use crate::generic;
use crate::midi::MidiMessage;

/// The `Handler` trait is to be implemented by all device modules.
pub trait Handler {
    /// Handler for Universal Device Inquiry and response messages
    fn midi_udi_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_udi_handler(ctx, midi_message)
    }

    /// Handler for incoming MIDI CC messages
    fn midi_cc_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_cc_in_handler(ctx, midi_message)
    }
    /// Handler for outgoing MIDI CC messages
    fn midi_cc_out_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_cc_out_handler(ctx, midi_message)
    }
    /// Handler for control change events sent by the `Controller`
    /// provided by the module
    fn cc_handler(&self, ctx: &Ctx, event: &ControlChangeEvent) {
        generic::cc_handler(ctx, event)
    }

    /// Handler for incoming MIDI PC messages
    fn midi_pc_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_pc_in_handler(ctx, midi_message)
    }
    /// Handler for outgoing MIDI PC messages
    fn midi_pc_out_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_pc_out_handler(ctx, midi_message)
    }
    /// Handler for program change events sent by the UI `Controller`
    fn pc_handler(&self, ctx: &Ctx, event: &ProgramChangeEvent) {
        generic::pc_handler(ctx, event)
    }

    /// Handler for buffer load requests (UI or MIDI)
    fn load_handler(&self, ctx: &Ctx, event: &BufferLoadEvent) {
        generic::load_handler(ctx, event)
    }
    /// Handler for buffer store requests (UI)
    fn store_handler(&self, ctx: &Ctx, event: &BufferStoreEvent) {
        generic::store_handler(ctx, event);
    }
    /// Handler for buffer data received through MIDI
    fn buffer_handler(&self, ctx: &Ctx, event: &BufferDataEvent) {
        generic::buffer_handler(ctx, event);
        generic::buffer_modified_handler(ctx, event);
    }

    /// Handler for program "modified" status changes
    fn modified_handler(&self, ctx: &Ctx, event: &ModifiedEvent) {
        generic::modified_handler(ctx, event)
    }

    /// Handler for incoming MIDI sysex messages
    fn midi_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_in_handler(ctx, midi_message)
    }
    /// Handler for outgoing MIDI sysex messages
    fn midi_out_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_out_handler(ctx, midi_message)
    }

    /// Handler for raw incoming MIDI byte data
    fn data_in_handler(&self, ctx: &Ctx, bytes: &Vec<u8>) {
        // todo
    }
    /// Handler for raw outgoing MIDI byte data
    fn data_out_handler(&self, ctx: &Ctx, bytes: &Vec<u8>) {
        // todo
    }

    /// Called when the device context is first initialized
    fn new_device_handler(&self, ctx: &Ctx) {
        generic::new_device_handler(ctx);
    }

    /// Handler for custom markers that this handler sent to itself
    fn marker_handler(&self, ctx: &Ctx, marker: u32) {}

    fn control_value_from_buffer(&self, controller: &mut Controller, name: &str, buffer: &[u8]) {}
    fn control_value_to_buffer(&self, controller: &Controller, name: &str, buffer: &mut [u8]) {}
}

pub type BoxedHandler = Box<dyn Handler + 'static + Send>;

