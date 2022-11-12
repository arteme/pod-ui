use crate::context::Ctx;
use crate::event::*;
use crate::midi::MidiMessage;

/// The `Handler` trait is to be implemented by all device modules.
pub trait Handler {
    /// Handler for Universal Device Inquiry and response messages
    fn midi_udi_handler(&self, ctx: &Ctx, midi_message: &MidiMessage);

    /// Handler for incoming MIDI CC messages
    fn midi_cc_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage);
    /// Handler for outgoing MIDI CC messages
    fn midi_cc_out_handler(&self, ctx: &Ctx, midi_message: &MidiMessage);
    /// Handler for control change events sent by the `Controller`
    /// provided by the module
    fn cc_handler(&self, ctx: &Ctx, event: &ControlChangeEvent);

    /// Handler for incoming MIDI PC messages
    fn midi_pc_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage);
    /// Handler for outgoing MIDI PC messages
    fn midi_pc_out_handler(&self, ctx: &Ctx, midi_message: &MidiMessage);
    /// Handler for program change events sent by the UI `Controller`
    fn pc_handler(&self, ctx: &Ctx, event: &ProgramChangeEvent);

    /// Handler for buffer load requests (UI or MIDI)
    fn load_handler(&self, ctx: &Ctx, event: &BufferLoadEvent);
    /// Handler for buffer store requests (UI)
    fn store_handler(&self, ctx: &Ctx, event: &BufferStoreEvent);
    /// Handler for buffer data received through MIDI
    fn buffer_handler(&self, ctx: &Ctx, event: &BufferDataEvent);

    /// Handler for program "modified" status changes
    fn modified_handler(&self, ctx: &Ctx, event: &ModifiedEvent);

    /// Handler for incoming MIDI sysex messages
    fn midi_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage);
    /// Handler for outgoing MIDI sysex messages
    fn midi_out_handler(&self, ctx: &Ctx, midi_message: &MidiMessage);

    /// Handler for raw incoming MIDI byte data
    fn data_in_handler(&self, ctx: &Ctx, bytes: &Vec<u8>);
    /// Handler for raw outgoing MIDI byte data
    fn data_out_handler(&self, ctx: &Ctx, bytes: &Vec<u8>);

    /// Called when the device context is first initialized
    fn new_device_handler(&self, ctx: &Ctx);
}

pub type BoxedHandler = Box<dyn Handler + 'static + Send + Sync>;

