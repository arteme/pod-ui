use crate::model::{Config, Control, AbstractControl};
use std::collections::HashMap;
use tokio::sync::broadcast;
use log::*;
use std::sync::{Mutex, Arc};

pub struct Controller {
    pub config: Config,
    values: HashMap<String, (u16, u8)>,

    tx: broadcast::Sender<String>,
    rx: broadcast::Receiver<String>
}

pub trait GetSet {
    fn has(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> Option<u16>;
    fn get_origin(&self, name: &str) -> Option<(u16,u8)>;
    fn set(&self, name: &str, value: u16, origin: u8) -> ();
    fn get_config(&self, name: &str) -> Option<Control>;
}

impl Controller {
    pub fn new(config: Config) -> Self {
        let mut values: HashMap<String, (u16, u8)> = HashMap::new();
        for (name, _) in config.controls.iter() {
            values.insert(name.clone(), (0, 0));
        }

        let (tx, rx) = broadcast::channel::<String>(16);

        Controller { config, values, tx, rx }
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.get(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<u16> {
        self.values.get(name).map(|v| v.0)
    }

    pub fn get_origin(&self, name: &str) -> Option<(u16, u8)> {
        self.values.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, value: u16, origin: u8) -> () {
        info!("set {:?} = {} <{}>", name, value, origin);
        let ref tx = self.tx;
        self.values.get_mut(name).map(|v| {
            if v.0 != value {
                v.0 = value;
                v.1 = origin;
                tx.send(name.to_string());
            }
        });
    }

    pub fn set_nosignal(&mut self, name: &str, value: u16, origin: u8) -> () {
        info!("set {:?} = {} <{}> (no signal)", name, value, origin);
        let ref tx = self.tx;
        self.values.get_mut(name).map(|mut v| {
            v.0 = value;
            v.1 = origin;
        });
    }

    pub fn get_config(&self, name: &str) -> Option<&Control> {
        self.config.controls.get(name)
    }

    pub fn get_config_by_cc(&self, cc: u8) -> Option<(&String, &Control)> {
        self.config.controls.iter().find(|&(_name, control)| {
            match control.get_cc() {
                Some(v) if v == cc => true,
                _ => false
            }
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

impl GetSet for Arc<Mutex<Controller>> {
    fn has(&self, name: &str) -> bool {
        let c = self.lock().unwrap();
        return c.has(name);
    }

    fn get(&self, name: &str) -> Option<u16> {
        let c = self.lock().unwrap();
        return c.get(name);
    }

    fn get_origin(&self, name: &str) -> Option<(u16, u8)> {
        let c = self.lock().unwrap();
        return c.get_origin(name);
    }

    fn set(&self, name: &str, value: u16, origin: u8) -> () {
        let mut c = self.lock().unwrap();
        c.set(&name, value, origin);
    }

    fn get_config(&self, name: &str) -> Option<Control> {
        let c = self.lock().unwrap();
        return c.get_config(name).map(|c| c.clone());
    }
}