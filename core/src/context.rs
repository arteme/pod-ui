use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};
use crate::controller::*;
use crate::dump::ProgramsDump;
use crate::edit::EditBuffer;
use crate::event::{EventSender, Origin, Program};
use crate::handler::BoxedHandler;
use crate::model::Config;

pub struct Ctx {
    pub config: &'static Config,

    pub handler: BoxedHandler,
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

    pub fn set_midi_channel(&self, midi_channel: u8) {
        self.ui_controller.set("midi_channel", midi_channel as u16, StoreOrigin::NONE);
    }

    pub fn program(&self) -> Program {
        self.ui_controller.get("program").unwrap().into()
    }

    pub fn set_program(&self, program: Program, origin: Origin) {
        self.ui_controller.set("program", program.into(), origin.into());
    }

    pub fn program_prev(&self) -> Program {
        self.ui_controller.get("program:prev").unwrap().into()
    }

    pub fn set_program_prev(&self, program: Program, origin: Origin) {
        self.ui_controller.set("program:prev", program.into(), origin.into());
    }

    pub fn program_num(&self) -> usize {
        self.ui_controller.get("program_num").unwrap() as usize
    }
}

impl Debug for Ctx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Ctx>")
    }
}
