use std::collections::HashMap;
use crate::controller::{Controller, Signal, Store};
use crate::model::{AbstractControl, Config, Control, VirtualSelect};

pub struct CCValues;

impl CCValues {
    pub fn generate_cc_controls(config: &Config) -> HashMap<String, Control> {
        let mut map = HashMap::new();
        for control in config.controls.values() {
            let Some(cc) = control.get_cc() else { continue };
            let name = format!("cc.{}", cc);
            map.insert(name, VirtualSelect {}.into());
        }

        map
    }

    pub fn set_cc_value(controller: &mut Controller, cc: u8, value: u8, origin: u8) {
        let name = format!("cc.{}", cc);
        controller.set_full(&name, value as u16, origin, Signal::None);
    }

    pub fn get_cc_value(controller: &Controller, cc: u8) -> Option<u8> {
        let name = format!("cc.{}", cc);
        controller.get(&name).map(|v| v as u8)
    }
}

pub trait CCAccess {
    fn set_cc_value(&mut self, cc: u8, value: u8, origin: u8);
    fn get_cc_value(&self, cc: u8) -> Option<u8>;
}

impl CCAccess for Controller {
    fn set_cc_value(&mut self, cc: u8, value: u8, origin: u8) {
        CCValues::set_cc_value( self ,cc, value, origin)
    }

    fn get_cc_value(&self, cc: u8) -> Option<u8> {
        CCValues::get_cc_value(self, cc)
    }
}
