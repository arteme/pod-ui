use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use pod_core::dump::ProgramsDump;
use pod_core::edit::EditBuffer;
use pod_core::model::Config;

pub fn empty_edit_buffer() -> EditBuffer {
    EditBuffer::new(&Config::empty())
}

pub fn empty_dump() -> ProgramsDump {
    ProgramsDump::new(&Config::empty())
}

pub fn empty_edit_buffer_arc() -> Arc<Mutex<EditBuffer>> {
    Arc::new(Mutex::new(empty_edit_buffer()))
}

pub fn empty_dump_arc() -> Arc<Mutex<ProgramsDump>> {
    Arc::new(Mutex::new(empty_dump()))
}
