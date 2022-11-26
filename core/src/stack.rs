use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use crate::controller::Controller;
use crate::store::*;

pub struct ControllerStack {
    controllers: Vec<Arc<Mutex<Controller>>>,
    tx: Option<broadcast::Sender<Event<String>>>
}

impl ControllerStack {
    pub fn new() -> Self {
        Self {
            controllers: vec![],
            tx: None
        }
    }

    pub fn with_broadcast(tx: broadcast::Sender<Event<String>>) -> Self {
        Self {
            controllers: vec![],
            tx: Some(tx)
        }
    }

    pub fn add(&mut self, controller: Arc<Mutex<Controller>>) {
        controller.broadcast(self.tx.as_ref().cloned());
        self.controllers.push(controller);
    }

    pub fn remove(&mut self, controller: Arc<Mutex<Controller>>) -> bool {
        let i = self.controllers.iter().enumerate()
            .find(|(i, c)| Arc::ptr_eq(*c, &controller) )
            .map(|(i,_)| i);
        if let Some(i) = i {
            let c = self.controllers.remove(i);
            c.broadcast(None);

            true
        } else {
            false
        }
    }

    pub fn controller_for(&self, key: &str) -> Option<&Arc<Mutex<Controller>>> {
        self.controllers.iter().find(|c| c.has(key))
    }
}

impl Store<&str, u16, String> for ControllerStack {
    fn has(&self, key: &str) -> bool {
        self.controller_for(key).is_some()
    }

    fn get(&self, key: &str) -> Option<u16> {
        self.controller_for(key)
            .and_then(|c| c.get(key))
    }

    fn set_full(&mut self, key: &str, value: u16, origin: Origin, signal: Signal) -> bool {
        self.controller_for(key)
            .map(|c| c.set_full(key, value, origin, signal))
            .unwrap_or(false)
    }

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<String>>>) {
        self.tx = tx;
        for c in self.controllers.iter() {
            c.broadcast(self.tx.as_ref().cloned());
        }
    }
}

