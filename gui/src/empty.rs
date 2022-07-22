use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use pod_core::dump::ProgramsDump;
use pod_core::edit::EditBuffer;
use pod_core::model::Config;

pub static EMPTY_CONFIG: Lazy<Config> = Lazy::new(|| Config::empty());

pub fn empty_edit_buffer() -> EditBuffer {
    EditBuffer::new(&EMPTY_CONFIG)
}

pub fn empty_dump() -> ProgramsDump {
    ProgramsDump::new(&EMPTY_CONFIG)
}

pub fn empty_edit_buffer_arc() -> Arc<Mutex<EditBuffer>> {
    Arc::new(Mutex::new(empty_edit_buffer()))
}

pub fn empty_dump_arc() -> Arc<Mutex<ProgramsDump>> {
    Arc::new(Mutex::new(empty_dump()))
}
