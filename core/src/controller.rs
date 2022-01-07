use crate::model::{Config, Control, AbstractControl};
use std::collections::HashMap;
use tokio::sync::broadcast;
use log::*;
use std::sync::{Mutex, Arc};
use crate::store::Store;

pub struct Controller {
    pub config: Config,
    values: HashMap<String, (u16, u8)>,

    tx: broadcast::Sender<String>,
    rx: broadcast::Receiver<String>
}

pub trait ControllerStoreExt {
    fn get_origin(&self, name: &str) -> Option<(u16,u8)>;
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

    pub fn get_origin(&self, name: &str) -> Option<(u16, u8)> {
        self.values.get(name).cloned()
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
}

impl Store<&str, u16, String> for Controller {
    fn has(&self, name: &str) -> bool {
        self.values.get(name).is_some()
    }

    fn get(&self, name: &str) -> Option<u16> {
        self.values.get(name).map(|v| v.0)
    }

    fn set(&mut self, name: &str, value: u16, origin: u8) -> () {
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

    fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

impl ControllerStoreExt for Arc<Mutex<Controller>> {
    fn get_origin(&self, name: &str) -> Option<(u16, u8)> {
        let c = self.lock().unwrap();
        c.get_origin(name)
    }

    fn get_config(&self, name: &str) -> Option<Control> {
        let c = self.lock().unwrap();
        c.get_config(name).map(|c| c.clone())
    }
}