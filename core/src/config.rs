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

pub fn config_for_id(family: u16, member: u16) -> Option<&'static Config> {
    configs().iter().find(|config| {
        family == config.family && member == config.member
    })
}

// Connection
pub const UNSET: u8 = 0;
pub const MIDI: u8 = 1;
pub const GUI: u8 = 2;