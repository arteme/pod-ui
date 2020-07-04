use crate::model::{Config, Control, GetCC};
use std::collections::HashMap;
use tokio::sync::broadcast;
use log::*;

pub struct Controller {
    config: Config,
    values: HashMap<String, u16>,

    tx: broadcast::Sender<String>,
    rx: broadcast::Receiver<String>
}

impl Controller {
    pub fn new(config: Config) -> Self {
        let mut values: HashMap<String, u16> = HashMap::new();
        for (name, _) in config.controls.iter() {
            values.insert(name.clone(), 0);
        }

        let (tx, rx) = broadcast::channel::<String>(16);

        Controller { config, values, tx, rx }
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.get(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<u16> {
        self.values.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, value: u16) -> () {
        info!("set {:?} = {}", name, value);
        let ref tx = self.tx;
        self.values.get_mut(name).map(|v| {
            if *v != value {
                *v = value;
                tx.send(name.to_string());
            }
        });
    }

    pub fn set_nosignal(&mut self, name: &str, value: u16) -> () {
        info!("set {:?} = {} (no signal)", name, value);
        let ref tx = self.tx;
        self.values.get_mut(name).map(|mut v| {
            *v = value;
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