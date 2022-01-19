use std::collections::HashMap;
use std::fmt;
use log::warn;

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
    pub clean: Option<EffectEntry>,
    pub delay: Option<EffectEntry>,
}

#[derive(Clone, Default, Debug)]
pub struct EffectEntry {
    pub id: u8,
    pub effect_tweak: String,
    pub controls: Vec<String>
}

#[derive(Clone, Debug)]
pub enum Control {
    SwitchControl(SwitchControl),
    RangeControl(RangeControl),
    Select(Select)
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

#[derive(Clone, Debug)]
pub struct SwitchControl { pub cc: u8, pub addr: u8 }
#[derive(Clone, Debug)]
pub struct RangeControl { pub cc: u8, pub addr: u8, pub bytes: u8, pub from: u8, pub to: u8,
    pub format: Format<Self> }
#[derive(Clone, Debug)]
pub struct Select { pub cc: u8, pub addr: u8,
    pub from_midi: Option<Vec<u16>>, pub to_midi: Option<Vec<u16>> }




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
        RangeControl { cc: 0, addr: 0, bytes: 1, from: 0, to: 127,
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
        Select { cc: 0, addr: 0, from_midi: None, to_midi: None }
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

// --

impl RangeControl {
    pub fn fmt_percent(&self, v: f64) -> String {
        let from = self.from as f64;
        let to = self.to as f64;
        format!("{:1.0}%", (v - from) * 100.0 / (to - from))
    }

    pub fn fmt_percent_signed(&self, v: f64) -> String {
        let from = self.from as f64;
        let to = self.to as f64;

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
    pub fn cc_to_control(&self, cc: u8) -> Option<(&String, &Control)> {
        self.controls.iter()
            .find(|&(_, control)| {
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
                    // here we specifically disregard the length and concentrate on the
                    // first byte of multi-byte controls
                    Some((a, _)) if a as usize == addr => true,
                    _ => false
                }
            })
    }

    pub fn addr_to_cc_iter(&self, addr: usize) -> impl Iterator<Item = u8> + '_ {
        self.addr_to_control_iter(addr)
            .flat_map(|(_, control)| control.get_cc())
    }

}