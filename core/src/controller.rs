use crate::model::{Control, AbstractControl};
use std::collections::HashMap;
use tokio::sync::broadcast;
use log::*;
use std::sync::{Mutex, Arc};
use crate::store::{Origin, StoreBase};

// re-export useful things from store
pub use crate::store::{Store, StoreSetIm, Signal, Event};
pub use crate::store::{Origin as StoreOrigin};

pub struct Controller {
    store: StoreBase<String, u16>,
    pub controls: HashMap<String, Control>,
    values: HashMap<String, (u16, Origin)>,
}

pub trait ControllerStoreExt {
    fn get_origin(&self, name: &str) -> Option<(u16, Origin)>;
    fn get_config(&self, name: &str) -> Option<Control>;
}

impl Controller {
    pub fn new(controls: HashMap<String, Control>) -> Self {
        let mut values: HashMap<String, (u16, Origin)> = HashMap::new();
        for (name, _) in controls.iter() {
            values.insert(name.clone(), (0, Origin::NONE));
        }

        Controller { store: StoreBase::new(), controls, values }
    }

    pub fn get_origin(&self, name: &str) -> Option<(u16, Origin)> {
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

    pub fn ordered_controls(&self) -> Vec<(String, Control)> {
        let mut refs = self.controls.iter()
            .filter(|(_,c)| c.get_addr().is_some())
            .map(|(n,c)| (n.clone(),c.clone())).collect::<Vec<_>>();
        refs.sort_by(|a,b| {
            Ord::cmp(&b.1.get_addr().unwrap().0, &a.1.get_addr().unwrap().0)
        });
        refs
    }
}

impl Store<&str, u16, String> for Controller {
    fn has(&self, name: &str) -> bool {
        self.values.get(name).is_some()
    }

    fn get(&self, name: &str) -> Option<u16> {
        self.values.get(name).map(|v| v.0)
    }

    fn set_full(&mut self, name: &str, value: u16, origin: Origin, signal: Signal) -> bool {
        info!("set {:?} = {} <{:?}>", name, value, origin);
        let store = &self.store;
        self.values.get_mut(name).map(|v| {
            let value_changed = v.0 != value;
            // need to check "signal == Force" because we're also setting origin here!
            if value_changed || signal == Signal::Force {
                v.0 = value;
                v.1 = origin;
            }

            store.send_signal(name.to_string(), value, value_changed, origin, signal);
            value_changed
        }).unwrap_or_else(|| {
            warn!("No control {:?} defined", name);
            false
        })
    }

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<String, u16>>>) {
        self.store.broadcast(tx)
    }
}

impl ControllerStoreExt for Arc<Mutex<Controller>> {
    fn get_origin(&self, name: &str) -> Option<(u16, Origin)> {
        let c = self.lock().unwrap();
        c.get_origin(name)
    }

    fn get_config(&self, name: &str) -> Option<Control> {
        let c = self.lock().unwrap();
        c.get_config(name).map(|c| c.clone())
    }
}