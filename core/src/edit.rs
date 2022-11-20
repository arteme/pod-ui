use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use crate::controller::Controller;
use crate::model::{AbstractControl, Config, Control};
use crate::store::{Event, Signal, Store, StoreSetIm};
use log::*;
use tokio::sync::broadcast;
use crate::cc_values::CCValues;
use crate::config::MIDI;
use crate::str_encoder::StrEncoder;

pub struct EditBuffer {
    controller: Arc<Mutex<Controller>>,
    raw: Arc<Mutex<Box<[u8]>>>,
    modified: bool,
    encoder: StrEncoder
}

pub type ControlFromBufferFn = fn(&mut Controller, &str, &[u8]);
pub type ControlToBufferFn = fn(&mut Controller, &str, &[u8]);

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

    /*
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

     */

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
        controller.set_full("name_change", 1, MIDI, Signal::Force);
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
    let value = control.value_to_buffer(value);

    let addr = control.get_addr();
    if addr.is_none() {
        return; // skip virtual controls
    }

    let (addr, len) = addr.unwrap();
    let addr = addr as usize;
    match len {
        1 => {
            if value > u8::MAX as u32 {
                warn!("Control {:?} value {} out of bounds!", name, value);
            }
            buffer[addr] = value as u8;
        }
        2 => {
            if value > u16::MAX as u32 {
                warn!("Control {:?} value {} out of bounds!", name, value);
            }
            buffer[addr] = ((value >> 8) & 0xff) as u8;
            buffer[addr + 1] = (value & 0xff) as u8;
        }
        4 => {
            buffer[addr] = ((value >> 24) & 0xff) as u8;
            buffer[addr + 1] = ((value >> 16) & 0xff) as u8;
            buffer[addr + 2] = ((value >> 8) & 0xff) as u8;
            buffer[addr + 3] = (value & 0xff) as u8;
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
            buffer[addr] as u32
        }
        2 => {
            let a = buffer[addr] as u32;
            let b = buffer[addr + 1] as u32;
            (a << 8) | b
        }
        4 => {
            let a = buffer[addr] as u32;
            let b = buffer[addr + 1] as u32;
            let c = buffer[addr + 2] as u32;
            let d = buffer[addr + 3] as u32;
            (a << 24) | (b << 16) | (c << 8)  | d
        }
        n => {
            error!("Control width {} not supported!", n);
            0u32

        }
    };
    let value = control.value_from_buffer(value);
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

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<String>>>) {
        self.controller.broadcast(tx)
    }
}