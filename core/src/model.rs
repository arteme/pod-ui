use std::collections::HashMap;
use std::fmt;
use bitflags::bitflags;
use log::warn;

bitflags! {
    pub struct DeviceFlags: u16 {
        /// POD 2.0 supports a manual mode (PC 0) which doesn't have a
        /// program dump, but operated on edit buffer alone. Pocket POD
        /// does not, PC 0 does nothing.
        /// Set if the device supports a manual mode.
        const MANUAL_MODE                        = 0x0001;
        /// When selecting a program that is marked as modified, Line6 Edit
        /// doesn't send a PC followed by an edit buffer dump. It sends an
        /// edit buffer dump only. Indeed, a PC followed by an edit buffer dump
        /// confuses POD 2.0, which switches to a completely different program
        /// altogether.
        /// When doing virtual editing in Vyzex, it will send a PC followed by
        /// edit buffer dump to Pocket POD, which processes them correctly.
        /// Set this flag to send PC + edit buffer dump.
        const MODIFIED_BUFFER_PC_AND_EDIT_BUFFER = 0x0002;
        /// When receiving an "all programs dump request" message, a POD 2.0
        /// will send an "all programs dump" message. A Pocket POD will send
        /// a set of "program patch dump" messages for each individual program.
        /// Set this flag for POD 2.0 behavior.
        const ALL_PROGRAMS_DUMP                  = 0x0004;
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub name: String,
    pub family: u16,
    pub member: u16,

    pub program_size: usize,
    pub program_num: usize,

    pub toggles: Vec<Toggle>,
    pub amp_models: Vec<Amp>,
    pub cab_models: Vec<String>,
    pub effects: Vec<Effect>,
    pub controls: HashMap<String, Control>,
    pub init_controls: Vec<String>,

    pub out_cc_edit_buffer_dump_req: Vec<u8>,
    pub in_cc_edit_buffer_dump_req: Vec<u8>,

    pub program_name_addr: usize,
    pub program_name_length: usize,
    pub flags: DeviceFlags
}


#[derive(Clone, Default, Debug)]
pub struct Amp {
    pub name: String,
    pub reverb: u16,
    pub bright_switch: bool,
    pub presence: bool,
    pub drive2: bool,
}

#[derive(Clone, Default, Debug)]
pub struct Effect {
    pub name: String,
    pub clean: Option<EffectEntry>,
    pub delay: Option<EffectEntry>,
}

#[derive(Clone, Default, Debug)]
pub struct EffectEntry {
    pub id: u8,
    pub effect_tweak: String,
    pub controls: Vec<String>
}

#[derive(Clone, Default, Debug)]
pub struct Toggle {
    pub name: String,
    pub position_control: String,
    pub on_position: usize,
    pub off_position: usize,
}

#[derive(Clone, Debug)]
pub enum Control {
    SwitchControl(SwitchControl),
    MidiSwitchControl(MidiSwitchControl),
    RangeControl(RangeControl),
    Select(Select),
    MidiSelect(MidiSelect),
    VirtualSelect(VirtualSelect),
    Button(Button)
}

#[derive(Clone)]
pub enum Format<T> {
    None,
    Callback(fn (&T, f64) -> String),
    Data(FormatData),
    Labels(Vec<String>)
}

impl<T> fmt::Debug for Format<T>  {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::None => write!(f, "<no format>"),
            Format::Callback(_) => write!(f, "<callback>"),
            Format::Data(_) => write!(f, "<data>"),
            Format::Labels(_) => write!(f, "<labels>")
        }
    }
}

/// v = kx + b
#[derive(Clone, Debug)]
pub struct FormatData {
    pub k: f64,
    pub b: f64,
    pub format: String
}

impl Default for FormatData {
    fn default() -> Self {
        FormatData { k: 1.0, b: 0.0, format: "{val}".into() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufferConfig {
    /// Value in program dump = switch control value
    Normal,
    /// Value in program dump = MIDI value of the control value
    Midi
}

#[derive(Clone, Debug)]
pub struct SwitchControl { pub cc: u8, pub addr: u8, pub inverted: bool, pub buffer_config: BufferConfig }

#[derive(Clone, Debug)]
pub struct MidiSwitchControl { pub cc: u8 }
#[derive(Clone, Debug)]
pub struct RangeControl { pub cc: u8, pub addr: u8, pub config: RangeConfig, pub format: Format<Self> }
#[derive(Clone, Debug)]
pub enum RangeConfig {
    Normal { buffer_config: BufferConfig },
    Short { from: u8, to: u8, buffer_config: BufferConfig },
    Long { from: u16, to: u16 },
    Function { from_midi: fn(u8) -> u16, to_midi: fn(u16) -> u8, buffer_config: BufferConfig },
    MultibyteHead { from: u16, to: u16, bitmask: u16, shift: u8,
        size: u8, from_buffer: fn(u32) -> u16, to_buffer: fn(u16) -> u32 },
    MultibyteTail { bitmask: u16, shift: u8 }
}

#[derive(Clone, Debug)]
pub struct Select { pub cc: u8, pub addr: u8 }
#[derive(Clone, Debug)]
pub struct MidiSelect { pub cc: u8 }
#[derive(Clone, Debug)]
pub struct VirtualSelect {}
#[derive(Clone, Debug)]
pub struct Button {}



impl Default for SwitchControl {
    fn default() -> Self {
        SwitchControl { cc: 0, addr: 0, inverted: false, buffer_config: BufferConfig::Normal }
    }
}

impl From<SwitchControl> for Control {
    fn from(c: SwitchControl) -> Self {
        Control::SwitchControl(c)
    }
}

impl Default for MidiSwitchControl {
    fn default() -> Self {
        MidiSwitchControl { cc: 0 }
    }
}

impl From<MidiSwitchControl> for Control {
    fn from(c: MidiSwitchControl) -> Self {
        Control::MidiSwitchControl(c)
    }
}

impl Default for RangeControl {
    fn default() -> Self {
        RangeControl { cc: 0, addr: 0, config: RangeConfig::Normal { buffer_config: BufferConfig::Normal },
            format: Format::None }
    }
}

impl From<RangeControl> for Control {
    fn from(c: RangeControl) -> Self {
        Control::RangeControl(c)
    }
}

impl Default for Select {
    fn default() -> Self {
        Select { cc: 0, addr: 0 }
    }
}

impl From<Select> for Control {
    fn from(c: Select) -> Self {
        Control::Select(c)
    }
}

impl Default for MidiSelect {
    fn default() -> Self {
        MidiSelect { cc: 0 }
    }
}

impl From<MidiSelect> for Control {
    fn from(c: MidiSelect) -> Self {
        Control::MidiSelect(c)
    }
}

impl Default for VirtualSelect {
    fn default() -> Self {
        VirtualSelect {}
    }
}

impl From<VirtualSelect> for Control {
    fn from(c: VirtualSelect) -> Self {
        Control::VirtualSelect(c)
    }
}

impl Default for Button {
    fn default() -> Self {
        Button {}
    }
}

impl From<Button> for Control {
    fn from(c: Button) -> Self {
        Control::Button(c)
    }
}

pub trait AbstractControl {
    fn get_cc(&self) -> Option<u8> { None }
    fn get_addr(&self) -> Option<(u8, u8)> { None }

    fn value_from_midi(&self, value: u8, _control_value: u16) -> u16 { value as u16 }
    fn value_to_midi(&self, value: u16) -> u8 { value as u8 }

    fn value_from_buffer(&self, value: u32) -> u16 { value as u16 }
    fn value_to_buffer(&self, value: u16) -> u32 { value as u32 }

}

impl AbstractControl for RangeControl {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
    fn get_addr(&self) -> Option<(u8, u8)> {
        let bytes = match self.config {
            RangeConfig::Long { .. } => 2,
            RangeConfig::MultibyteHead { size, .. } => size,
            _ => 1
        };
        Some((self.addr, bytes))
    }

    fn value_from_midi(&self, value: u8, control_value: u16) -> u16 {
        match &self.config {
            RangeConfig::Short { from, to, .. } => {
                let scale = 127 / (to - from);
                (value / scale + from) as u16
            }
            RangeConfig::Long { from, to } => {
                let (from, to) = (*from as f64, *to as f64);
                let scale = (to - from) / 127.0;
                let v = value as f64 * scale + from;
                v.min(to).max(from) as u16
            }
            RangeConfig::MultibyteHead { bitmask, shift, .. } |
            RangeConfig::MultibyteTail { bitmask, shift, .. } => {
                let mask = bitmask << shift;
                (control_value & !mask) | ((value as u16 & bitmask) << shift)
            },
            RangeConfig::Function { from_midi, .. } => {
                from_midi(value)
            }
            _ => value as u16
        }
    }

    fn value_to_midi(&self, value: u16) -> u8 {
        match &self.config {
            RangeConfig::Short { from, to, .. } => {
                let scale = 127 / (to - from);
                (value as u8 - from) * scale
            }
            RangeConfig::Long { from, to } => {
                let (from, to) = (*from as f64, *to as f64);
                let scale = (to - from) / 127.0;
                let v = (value as f64 - from) / scale;
                v.min(127.0).max(0.0) as u8
            }
            RangeConfig::MultibyteHead { bitmask, shift, .. } |
            RangeConfig::MultibyteTail { bitmask, shift } => {
                let v = (value >> shift) & bitmask;
                v.min(127).max(0) as u8
            }
            RangeConfig::Function { to_midi, .. } => {
                to_midi(value)
            }
            _ => value as u8
        }
    }

    fn value_from_buffer(&self, value: u32) -> u16 {
        match &self.config {
            RangeConfig::MultibyteHead { from_buffer, .. } => {
                from_buffer(value)
            }
            RangeConfig::Function { buffer_config, .. } if *buffer_config == BufferConfig::Midi => {
                self.value_from_midi(value as u8, 0)
            }
            RangeConfig::Normal { buffer_config, .. } if *buffer_config == BufferConfig::Midi => {
                self.value_from_midi(value as u8, 0)
            }
            RangeConfig::Short { buffer_config, .. } if *buffer_config == BufferConfig::Midi => {
                self.value_from_midi(value as u8, 0)
            }
            _ => {
                value as u16
            }
        }
    }

    fn value_to_buffer(&self, value: u16) -> u32 {
        match &self.config {
            RangeConfig::MultibyteHead { to_buffer, .. } => {
                to_buffer(value)
            }
            RangeConfig::Function { buffer_config, .. } if *buffer_config == BufferConfig::Midi => {
                self.value_to_midi(value) as u32
            }
            RangeConfig::Normal { buffer_config, .. } if *buffer_config == BufferConfig::Midi => {
                self.value_to_midi(value) as u32
            }
            RangeConfig::Short { buffer_config, .. } if *buffer_config == BufferConfig::Midi => {
                self.value_to_midi(value) as u32
            }
            _ => {
                value as u32
            }
        }

    }
}

impl AbstractControl for SwitchControl {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
    fn get_addr(&self) -> Option<(u8, u8)> { Some((self.addr, 1)) }

    fn value_from_midi(&self, value: u8, _control_value: u16) -> u16 {
        let value = value > 63;
        (self.inverted ^ value) as u16
    }

    fn value_to_midi(&self, value: u16) -> u8 {
        let value = value > 0;
        if self.inverted ^ value { 127 } else { 0 }
    }

    fn value_from_buffer(&self, value: u32) -> u16 {
        match &self.buffer_config {
            BufferConfig::Normal => {
                let value = value != 0;
                (self.inverted ^ value) as u16
            }
            BufferConfig::Midi => {
                self.value_from_midi(value as u8, 0)
            }
        }
    }

    fn value_to_buffer(&self, value: u16) -> u32 {
        match &self.buffer_config {
            BufferConfig::Normal => {
                let value = value != 0;
                (self.inverted ^ value) as u32
            }
            BufferConfig::Midi => {
                self.value_to_midi(value) as u32
            }
        }
    }
}

impl AbstractControl for MidiSwitchControl {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }

    fn value_from_midi(&self, value: u8, _control_value: u16) -> u16 {
        value as u16 / 64
    }

    fn value_to_midi(&self, value: u16) -> u8 {
        if value > 0 { 127 } else { 0 }
    }
}

impl AbstractControl for Select {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
    fn get_addr(&self) -> Option<(u8, u8)> { Some((self.addr, 1)) }
}

impl AbstractControl for MidiSelect {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
}

impl AbstractControl for VirtualSelect {}

impl AbstractControl for Button {}

impl Control {
    fn abstract_control(&self) -> &dyn AbstractControl {
        match self {
            Control::SwitchControl(c) => c,
            Control::MidiSwitchControl(c) => c,
            Control::RangeControl(c) => c,
            Control::Select(c) => c,
            Control::MidiSelect(c) => c,
            Control::VirtualSelect(c) => c,
            Control::Button(c) => c
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

    fn value_from_midi(&self, value: u8, control_value: u16) -> u16 {
        self.abstract_control().value_from_midi(value, control_value)
    }

    fn value_to_midi(&self, value: u16) -> u8 {
        self.abstract_control().value_to_midi(value)
    }

    fn value_from_buffer(&self, value: u32) -> u16 {
        self.abstract_control().value_from_buffer(value)
    }

    fn value_to_buffer(&self, value: u16) -> u32 {
        self.abstract_control().value_to_buffer(value)
    }
}

// --

impl RangeControl {
    pub fn bounds(&self) -> (f64, f64) {
        match self.config {
            RangeConfig::Normal { .. } => (0.0, 127.0),
            RangeConfig::Short { from, to, .. } => (from as f64, to as f64),
            RangeConfig::Function { from_midi, .. } => {
                let a = from_midi(0) as f64;
                let b = from_midi(127) as f64;
                (a.min(b), a.max(b))
            }
            RangeConfig::Long { from, to } |
            RangeConfig::MultibyteHead { from, to, .. } => (from as f64, to as f64),
            RangeConfig::MultibyteTail { .. } => (0.0, 0.0)
        }
    }

    pub fn fmt_percent(&self, v: f64) -> String {
        let (from, to) = self.bounds();
        format!("{:1.0}%", (v - from) * 100.0 / (to - from))
    }

    pub fn fmt_percent_signed(&self, v: f64) -> String {
        let (from, to) = self.bounds();

        let n = ((to - from) / 2.0).floor();
        let p = ((to - from) / 2.0).ceil();

        let v1 = if v <= n { v - n } else { v - p };
        format!("{:1.0}%", v1 * 100.0 / n)
    }
}

impl FormatData {
    pub fn format(&self, v: f64) -> String {
        let val = self.k * v + self.b;

        let mut vars: HashMap<String, f64> = HashMap::new();
        vars.insert("val".into(), val);

        let f = |mut fmt: strfmt::Formatter| {
            fmt.f64(*vars.get(fmt.key).unwrap())
        };

        strfmt::strfmt_map(&self.format, &f)
            .unwrap_or_else(|err| {
                // TODO: format failed for which widget?
                warn!("Format failed: {}", err);
                "".into()
            })
    }
}

// ---

impl Config {
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            family: 0,
            member: 0,
            program_size: 0,
            program_num: 0,
            toggles: vec![],
            amp_models: vec![],
            cab_models: vec![],
            effects: vec![],
            controls: Default::default(),
            init_controls: vec![],
            out_cc_edit_buffer_dump_req: vec![],
            in_cc_edit_buffer_dump_req: vec![],
            program_name_addr: 0,
            program_name_length: 0,
            flags: DeviceFlags::empty()
        }
    }

    pub fn cc_to_control(&self, cc: u8) -> Option<(&String, &Control)> {
        self.controls.iter()
            .find(|(_, control)| {
                match control.get_cc() {
                    Some(v) if v == cc => true,
                    _ => false
                }
            })
    }

    pub fn cc_to_addr(&self, cc: u8) -> Option<usize> {
        self.cc_to_control(cc)
            .and_then(|(_, control)| control.get_addr())
            .map(|(addr, _)| addr as usize)
    }

    pub fn addr_to_control_iter(&self, addr: usize) -> impl Iterator<Item = (&String, &Control)>  {
        self.controls.iter()
            .filter(move |(_, control)| {
                match control.get_addr() {
                    Some((a, n)) if (a..a+n).contains(&(addr as u8)) => true,
                    _ => false
                }
            })
    }

    pub fn addr_to_cc_iter(&self, addr: usize) -> impl Iterator<Item = u8> + '_ {
        self.addr_to_control_iter(addr)
            .filter(move |(_, control)| {
                match control.get_addr() {
                    // Only interested in controls' fist byte that maps to a CC
                    Some((a, _)) if a == addr as u8 => true,
                    _ => false
                }
            })
            .flat_map(|(_, control)| control.get_cc())
    }

    pub fn addr_to_control_vec(&self, addr: usize, reverse: bool) -> Vec<(&String, &Control)>  {
        let mut controls = self.addr_to_control_iter(addr).collect::<Vec<_>>();
        controls.sort_by_key(|(_, c)| c.get_addr().map(|(a, _)| a).unwrap_or_default());
        if reverse { controls.reverse(); }
        controls
    }

}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.family == other.family && self.member == other.member
    }
}