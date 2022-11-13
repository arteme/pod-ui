use std::cell::Cell;
use pod_core::handler::Handler;
use pod_core::midi::MidiMessage;

pub(crate) struct PodXtHandler {
    midi_out_queue: Cell<Vec<MidiMessage>>
}

impl PodXtHandler {
    pub fn new() -> Self {
        Self {
            midi_out_queue: Cell::new(vec![])
        }
    }
}

impl Handler for PodXtHandler {

}