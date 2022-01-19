use crate::model::Config;

// TODO: remove the "static mut" hack!
pub static mut PODS: Vec<Config> = Vec::new();

pub fn register_config(config: &Config) {
    unsafe {
        PODS.push(config.clone());
    }
}

pub fn configs() -> &'static Vec<Config> {
    unsafe {
        return &PODS;
    }
}

// Connection
pub const UNSET: u8 = 0;
pub const MIDI: u8 = 1;
pub const GUI: u8 = 2;