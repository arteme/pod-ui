use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use crate::controller::Controller;
use crate::model::{AbstractControl, Config, Control};
use crate::store::*;
use crate::cc_values::CCValues;
use crate::str_encoder::StrEncoder;

pub struct EditBuffer {
    controller: Arc<Mutex<Controller>>,
    raw: Arc<Mutex<Box<[u8]>>>,
    modified: bool,
    encoder: StrEncoder
}

impl EditBuffer {
    pub fn new(config: &Config) -> Self {

        let controls = config.controls.clone().into_iter()
            .chain(CCValues::generate_cc_controls(config).into_iter())
            .collect::<HashMap<_,_>>();

        let controller = Controller::new(controls);
        let raw = vec![0u8; config.program_size].into_boxed_slice();

        let controller = Arc::new(Mutex::new(controller));
        let raw = Arc::new(Mutex::new(raw));
        let encoder = StrEncoder::new(&config);

        Self { controller, raw, encoder, modified: false }
    }

    pub fn controller(&self) -> Arc<Mutex<Controller>> {
        self.controller.clone()
    }

    pub fn controller_locked(&self) -> MutexGuard<'_, Controller> {
        self.controller.lock().unwrap()
    }

    pub fn raw_locked(&self) -> MutexGuard<'_, Box<[u8]>> {
        self.raw.lock().unwrap()
    }

    pub fn load_from_raw<F>(&mut self, control_value_from_buffer: F)
        where F: Fn(&mut Controller, &str, &[u8])
    {
        let mut controller = self.controller.lock().unwrap();
        let raw = self.raw.lock().unwrap();
        for (name, _) in ordered_controls(&controller) {
            control_value_from_buffer(&mut controller, &name, &raw);
        }
        controller.set_full("name_change", 1, Origin::NONE, Signal::Force);
    }

    pub fn name(&self) -> String {
        let raw = self.raw.lock().unwrap();
        self.encoder.str_from_buffer(&raw)
    }

    pub fn set_name(&mut self, str: &str) {
        let mut raw = self.raw.lock().unwrap();
        let modified = self.encoder.str_from_buffer(&raw).as_str() != str.trim();
        if modified {
            self.encoder.str_to_buffer(str, &mut raw);
            self.controller.set_full("name_change", 1, Origin::UI, Signal::Force);
            self.modified = true;
        }
    }

    pub fn modified(&self) -> bool {
        self.modified
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified
    }
}

fn ordered_controls(controller: &Controller) -> Vec<(String, Control)> {
    let mut refs = controller.controls.iter()
        .filter(|(_,c)| c.get_addr().is_some())
        .map(|(n,c)| (n.clone(),c.clone())).collect::<Vec<_>>();
    refs.sort_by(|a,b| {
        Ord::cmp(&b.1.get_addr().unwrap().0, &a.1.get_addr().unwrap().0)
    });
    refs
}