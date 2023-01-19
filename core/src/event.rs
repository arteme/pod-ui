use std::fmt::Debug;
use log::warn;
use tokio::sync::broadcast;
use crate::midi::MidiMessage;
use crate::store::{Origin as StoreOrigin};

#[derive(Clone, Debug, PartialEq)]
pub enum Program {
    ManualMode,
    Tuner,
    Program(u16)
}

impl From<u16> for Program {
    fn from(v: u16) -> Self {
        match v {
            999 => Program::Tuner,
            998 => Program::ManualMode,
            v => Program::Program(v)
        }
    }
}

impl Into<u16> for Program {
    fn into(self) -> u16 {
        match self {
            Program::Tuner => 999,
            Program::ManualMode => 998,
            Program::Program(v) => v
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Origin {
    MIDI,
    UI,
}

impl Into<StoreOrigin> for Origin {
    fn into(self) -> StoreOrigin {
        match self {
            Origin::MIDI => StoreOrigin::MIDI,
            Origin::UI => StoreOrigin::UI
        }
    }
}

impl TryFrom<StoreOrigin> for Origin {
    type Error = &'static str;

    fn try_from(value: StoreOrigin) -> Result<Self, Self::Error> {
        match value {
            StoreOrigin::NONE => Err("Unsupported origin"),
            StoreOrigin::MIDI => Ok(Origin::MIDI),
            StoreOrigin::UI => Ok(Origin::UI),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ControlChangeEvent {
    pub name: String,
    pub value: u16,
    /// This is intentionally StoreOrigin, not Origin, to detect buffer loads
    pub origin: StoreOrigin,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Buffer {
    EditBuffer,
    Current,
    Program(usize),
    All
}

#[derive(Clone, Debug)]
pub struct ProgramChangeEvent {
    pub program: Program,
    pub origin: Origin,
}

#[derive(Clone, Debug)]
pub struct BufferLoadEvent {
    pub buffer: Buffer,
    pub origin: Origin,
}

#[derive(Clone, Debug)]
pub struct BufferStoreEvent {
    pub buffer: Buffer,
    pub origin: Origin,
}

#[derive(Clone, Debug)]
pub struct BufferDataEvent {
    pub buffer: Buffer,
    pub origin: Origin,
    pub request: Origin,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ModifiedEvent {
    pub buffer: Buffer,
    pub origin: Origin,
    pub modified: bool
}

#[derive(Clone, Debug)]
pub struct DeviceDetectedEvent {
    pub name: String,
    pub version: String
}

#[derive(Clone, Debug)]
pub struct NewConfigEvent {
    pub midi_changed: bool,
    pub midi_channel: u8,
    pub config_changed: bool
}

// -------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum AppEvent {
    MidiIn(Vec<u8>),
    MidiOut(Vec<u8>),

    MidiMsgIn(MidiMessage),
    MidiMsgOut(MidiMessage),

    ControlChange(ControlChangeEvent),
    ProgramChange(ProgramChangeEvent),
    Load(BufferLoadEvent),
    Store(BufferStoreEvent),
    BufferData(BufferDataEvent),
    Modified(ModifiedEvent),

    DeviceDetected(DeviceDetectedEvent),
    NewConfig(NewConfigEvent),
    NewCtx,
    Shutdown,
    Notification(String),

    Marker(u32)
}

pub fn is_system_app_event(event: &AppEvent) -> bool {
    match event {
        AppEvent::DeviceDetected(_) | AppEvent::Notification(_) |
        AppEvent::NewConfig(_) | AppEvent::NewCtx | AppEvent::Shutdown => true,
        _ => false
    }
}

pub type EventSender = broadcast::Sender<AppEvent>;

pub trait SenderExt<T> {
    fn send_or_warn(&self, msg: T);
}

impl <T: Debug> SenderExt<T> for broadcast::Sender<T> {
    fn send_or_warn(&self, msg: T) {
        self.send(msg).unwrap_or_else(|err| {
            warn!("Message cannot be sent: {:?}", err.0);
            0
        });
    }
}
