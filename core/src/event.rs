use bytes::Bytes;
use log::warn;
use tokio::sync::broadcast;
use crate::context::Ctx;
use crate::midi::MidiMessage;

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

#[derive(Clone, Debug, PartialEq)]
pub enum Origin {
    MIDI,
    UI,
}

#[derive(Clone, Debug)]
pub struct ControlChangeEvent {
    pub name: String,
    pub value: u16,
    pub origin: Origin,
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

// -------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum AppEvent {
    MidiIn(Bytes),
    MidiOut(Bytes),

    MidiMsgIn(MidiMessage),
    MidiMsgOut(MidiMessage),

    ControlChange(ControlChangeEvent),
    ProgramChange(ProgramChangeEvent),
    Load(BufferLoadEvent),
    Store(BufferStoreEvent),
    BufferData(BufferDataEvent),
    Modified(ModifiedEvent),

    DeviceDetected(DeviceDetectedEvent),
    NewConfig,
    NewCtx(Ctx),
    Shutdown,
}

pub fn is_system_app_event(event: &AppEvent) -> bool {
    match event {
        AppEvent::DeviceDetected(_) |
        AppEvent::NewConfig | AppEvent::NewCtx(_) | AppEvent::Shutdown => true,
        _ => false
    }
}

pub type EventSender = broadcast::Sender<AppEvent>;

pub trait EventSenderExt {
    fn send_or_warn(&self, msg: AppEvent);
}

impl EventSenderExt for EventSender {
    fn send_or_warn(&self, msg: AppEvent) {
        self.send(msg).unwrap_or_else(|err| {
            warn!("Message cannot be sent: {:?}", err.0);
            0
        });
    }
}
