use crate::model::{Control, AbstractControl};
use std::collections::HashMap;
use tokio::sync::broadcast;
use log::*;
use std::sync::{Mutex, Arc};
use crate::store::*;

pub struct Controller {
    store: StoreBase<String>,
    pub controls: HashMap<String, Control>,
    values: HashMap<String, (u16, u8)>,
}

pub trait ControllerStoreExt {
    fn get_origin(&self, name: &str) -> Option<(u16,u8)>;
    fn get_config(&self, name: &str) -> Option<Control>;
}

impl Controller {
    pub fn new(controls: HashMap<String, Control>) -> Self {
        let mut values: HashMap<String, (u16, u8)> = HashMap::new();
        for (name, _) in controls.iter() {
            values.insert(name.clone(), (0, 0));
        }

        Controller { store: StoreBase::new(), controls, values }
    }

    pub fn get_origin(&self, name: &str) -> Option<(u16, u8)> {
        self.values.get(name).cloned()
    }

    pub fn get_config(&self, name: &str) -> Option<&Control> {
        self.controls.get(name)
    }

    pub fn get_config_by_cc(&self, cc: u8) -> Option<(&String, &Control)> {
        self.controls.iter().find(|&(_name, control)| {
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

    fn set_full(&mut self, name: &str, value: u16, origin: u8, signal: Signal) -> bool {
        info!("set {:?} = {} <{}>", name, value, origin);
        let store = &self.store;
        self.values.get_mut(name).map(|v| {
            let value_changed = v.0 != value;
            // need to check "signal == Force" because we're also setting origin here!
            if value_changed || signal == Signal::Force {
                v.0 = value;
                v.1 = origin;
            }

            store.send_signal(name.to_string(), value_changed, origin, signal);
            value_changed
        }).unwrap_or(false)
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<String>> {
        self.store.subscribe()
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