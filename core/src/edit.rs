use std::sync::{Arc, Mutex, MutexGuard};
use crate::controller::Controller;
use crate::model::{AbstractControl, Config, Control};
use crate::store::{Event, Store};
use log::*;
use tokio::task::JoinHandle;

pub struct EditBuffer {
    controller: Arc<Mutex<Controller>>,
    raw: Arc<Mutex<Box<[u8]>>>,
    pub name: String,
    pub modified: bool
}

impl EditBuffer {
    pub fn new(config: &Config) -> Self {
        let controller = Controller::new(config.controls.clone());
        let raw = vec![0u8; config.program_size].into_boxed_slice();

        let controller = Arc::new(Mutex::new(controller));
        let raw = Arc::new(Mutex::new(raw));

        Self {
            controller,
            raw,
            name: String::default(),
            modified: false
        }
    }

    pub fn start_thread(&self) -> JoinHandle<()> {
        let controller = self.controller.clone();
        let raw = self.raw.clone();
        let mut rx = controller.subscribe();

        tokio::spawn(async move {
            loop {
                let name = match rx.recv().await {
                    Ok(Event { key: name, .. }) => { name }
                    Err(e) => {
                        error!("Error in edit buffer 'controller -> raw' rx: {}", e);
                        String::default()
                    }
                };
                if name.is_empty() {
                    return;
                }

                let controller = controller.lock().unwrap();
                let mut raw = raw.lock().unwrap();

                control_value_to_buffer(&controller, &name, &mut raw);
            }
        })
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

    pub fn load_from_raw(&mut self, origin: u8) {
        let mut controller = self.controller.lock().unwrap();
        let raw = self.raw.lock().unwrap();
        for (name, _) in ordered_controls(&controller) {
            control_value_from_buffer(&mut controller, &name, &raw, origin);
        }
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

fn control_value_to_buffer(controller: &Controller, name: &str, buffer: &mut [u8]) {
    let control = controller.get_config(name);
    if control.is_none() {
        return;
    }
    let control = control.unwrap();
    let value = controller.get(name).unwrap();

    let addr = control.get_addr();
    if addr.is_none() {
        return; // skip virtual controls
    }

    let (addr, len) = addr.unwrap();
    let addr = addr as usize;
    match len {
        1 => {
            if value > u8::MAX as u16 {
                warn!("Control {:?} value {} out of bounds!", name, value);
            }
            buffer[addr] = value as u8;
        }
        2 => {
            buffer[addr] = ((value >> 8) & 0xff) as u8;
            buffer[addr + 1] = (value & 0xff) as u8;
        }
        n => {
            error!("Control width {} not supported!", n)
        }
    }
}
fn control_value_from_buffer(controller: &mut Controller, name: &str, buffer: &[u8], origin: u8) {
    let control = controller.get_config(name);
    if control.is_none() {
        return;
    }
    let control = control.unwrap();

    let addr = control.get_addr();
    if addr.is_none() {
        return; // skip virtual controls
    }
    let (addr, len) = control.get_addr().unwrap();
    let addr = addr as usize;
    let value = match len {
        1 => {
            buffer[addr] as u16
        }
        2 => {
            let a = buffer[addr] as u16;
            let b = buffer[addr + 1] as u16;
            (a << 8) | b
        }
        n => {
            error!("Control width {} not supported!", n);
            0u16
        }
    };
    controller.set(&name, value, origin);
}
