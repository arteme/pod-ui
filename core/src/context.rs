use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};
use crate::controller::*;
use crate::dump::ProgramsDump;
use crate::edit::EditBuffer;
use crate::event::EventSender;
use crate::model::Config;

#[derive(Clone)]
pub struct Ctx {
    pub config: &'static Config,

    pub controller: Arc<Mutex<Controller>>,
    pub edit: Arc<Mutex<EditBuffer>>,
    pub dump: Arc<Mutex<ProgramsDump>>,

    pub ui_controller: Arc<Mutex<Controller>>,

    pub app_event_tx: EventSender
}

impl Ctx {
    pub fn midi_channel(&self) -> u8 {
        self.ui_controller.get("midi_channel").unwrap() as u8
    }
}

impl Debug for Ctx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Ctx>")
    }
}