use std::sync::{Arc, Mutex, MutexGuard};
use crate::controller::Controller;
use crate::model::{AbstractControl, Config, Control};
use crate::store::{Event, Signal, Store, StoreSetIm};
use log::*;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use crate::str_encoder::StrEncoder;

pub struct EditBuffer {
    controller: Arc<Mutex<Controller>>,
    raw: Arc<Mutex<Box<[u8]>>>,
    modified: bool,
    encoder: StrEncoder
}

impl EditBuffer {
    pub fn new(config: &Config) -> Self {
        let controller = Controller::new(config.controls.clone());
        let raw = vec![0u8; config.program_size].into_boxed_slice();

        let controller = Arc::new(Mutex::new(controller));
        let raw = Arc::new(Mutex::new(raw));
        let encoder = StrEncoder::new(&config);

        Self { controller, raw, encoder, modified: false }
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
        controller.set_full("name_change", 1, origin, Signal::Force);
    }

    pub fn name(&self) -> String {
        let raw = self.raw.lock().unwrap();
        self.encoder.str_from_buffer(&raw)
    }

    pub fn set_name(&mut self, str: &str) {
        let mut  raw = self.raw.lock().unwrap();
        self.encoder.str_to_buffer(str, &mut raw);
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

impl Store<&str, u16, String> for EditBuffer {
    fn has(&self, name: &str) -> bool {
        self.controller.has(name)
    }

    fn get(&self, name: &str) -> Option<u16> {
        self.controller.get(name)
    }

    fn set_full(&mut self, name: &str, value: u16, origin: u8, signal: Signal) -> bool {
        self.controller.set_full(name, value, origin, signal)
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<String>> {
        self.controller.subscribe()
    }
}