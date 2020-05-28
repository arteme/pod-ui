use std::collections::HashMap;

#[derive(Debug)]
pub struct Config {
    pub name: String,
    pub family: u16,
    pub member: u16,

    pub program_size: usize,
    pub all_programs_size: usize,
    pub pod_id: u8, // used in sysex dump messages

    pub amp_models: Vec<String>,
    pub cab_models: Vec<String>,
    pub controls: HashMap<String, Control>
}

#[derive(Debug)]
pub enum Control {
    SwitchControl {
        cc: u8
    },
    RangeControl {
        cc: u8,
        from: u8,
        to: u8
    }
}