use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Config {
    pub name: String,
    pub family: u16,
    pub member: u16,

    pub program_size: usize,
    pub all_programs_size: usize,
    pub pod_id: u8, // used in sysex dump messages

    pub amp_models: Vec<Amp>,
    pub cab_models: Vec<String>,
    pub effects: Vec<Effect>,
    pub controls: HashMap<String, Control>,
    pub init_controls: Vec<String>,

    pub program_name_addr: usize,
    pub program_name_length: usize
}

#[derive(Clone, Default, Debug)]
pub struct Amp {
    pub name: String,
    pub bright_switch: bool,
    pub presence: bool,
    pub delay2: bool,
}

#[derive(Clone, Default, Debug)]
pub struct Effect {
    pub name: String,
    pub delay: Option<bool>,
}


#[derive(Clone, Debug)]
pub enum Control {
    SwitchControl(SwitchControl),
    RangeControl(RangeControl),
    Select(Select)
}

#[derive(Clone, Debug)]
pub struct SwitchControl { pub cc: u8, pub addr: u8 }
#[derive(Clone, Debug)]
pub struct RangeControl { pub cc: u8, pub addr: u8, pub bytes: u8, pub from: u8, pub to: u8 }
#[derive(Clone, Debug)]
pub struct Select { pub cc: u8, pub addr: u8 }

impl Default for SwitchControl {
    fn default() -> Self {
        SwitchControl { cc: 0, addr: 0 }
    }
}

impl From<SwitchControl> for Control {
    fn from(c: SwitchControl) -> Self {
        Control::SwitchControl(c)
    }
}

impl Default for RangeControl {
    fn default() -> Self {
        RangeControl { cc: 0, addr: 0, bytes: 1, from: 0, to: 127 }
    }
}
impl From<RangeControl> for Control {
    fn from(c: RangeControl) -> Self {
        Control::RangeControl(c)
    }
}

impl From<Select> for Control {
    fn from(c: Select) -> Self {
        Control::Select(c)
    }
}

pub trait AbstractControl {
    fn get_cc(&self) -> Option<u8>;
    fn get_addr(&self) -> Option<(u8, u8)>;
}

impl AbstractControl for RangeControl {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
    fn get_addr(&self) -> Option<(u8, u8)> { Some((self.addr, self.bytes)) }
}

impl AbstractControl for SwitchControl {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
    fn get_addr(&self) -> Option<(u8, u8)> { Some((self.addr, 1)) }
}

impl AbstractControl for Select {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
    fn get_addr(&self) -> Option<(u8, u8)> { Some((self.addr, 1)) }
}

impl Control {
    fn abstract_control(&self) -> &dyn AbstractControl {
        match self {
            Control::SwitchControl(c) => c,
            Control::RangeControl(c) => c,
            Control::Select(c) => c
        }
    }
}


impl AbstractControl for Control {
    fn get_cc(&self) -> Option<u8> {
        self.abstract_control().get_cc()
    }

    fn get_addr(&self) -> Option<(u8, u8)> {
        self.abstract_control().get_addr()
    }
}
