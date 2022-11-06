use tokio::sync::broadcast;
use std::sync::{Mutex, Arc};
use log::warn;

#[derive(Clone, PartialEq)]
pub enum Signal {
    None,
    Change,
    Force
}

#[derive(Clone)]
pub struct Event<K: Clone> {
    pub key: K,
    pub origin: u8,
    pub signal: Signal
}

pub struct StoreBase<K: Clone> {
    tx: Option<broadcast::Sender<Event<K>>>
}

impl <K: Clone> StoreBase<K> {
    pub fn new() -> Self {
        StoreBase { tx: None }
    }

    pub fn send_signal(&self, key: K, value_changed: bool, origin: u8, signal: Signal) -> () {
        let send = match signal {
            Signal::Force => true,
            Signal::Change if value_changed => true,
            _ => false
        };
        if send {
            let event = Event { key, origin, signal };
            if let Some(tx) = &self.tx {
                tx.send(event)
                    .map_err(|e| warn!("Failed to store event signal"))
                    .unwrap_or_default();
            }
        }
    }

    pub fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<K>>>) {
        self.tx = tx;
    }
}


pub trait Store<K, V, E: Clone> {
    fn has(&self, key: K) -> bool;
    fn get(&self, key: K) -> Option<V>;
    fn set_full(&mut self, key: K, value: V, origin: u8, signal: Signal) -> bool;
    fn set(&mut self, key: K, value: V, origin: u8) -> bool {
        self.set_full(key, value, origin, Signal::Change)
    }

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<E>>>);
}

pub trait StoreSetIm<K, V, E: Clone> {
    fn set_full(&self, key: K, value: V, origin: u8, signal: Signal) -> bool;
    fn set(&self, key: K, value: V, origin: u8) -> bool {
        self.set_full(key, value, origin, Signal::Change)
    }

    fn broadcast(&self, tx: Option<broadcast::Sender<Event<E>>>);
}

impl<K, V, E: Clone, T: Store<K,V,E>> Store<K, V, E> for Arc<Mutex<T>> {
    fn has(&self, key: K) -> bool {
        let s = self.lock().unwrap();
        s.has(key)
    }

    fn get(&self, key: K) -> Option<V> {
        let s = self.lock().unwrap();
        s.get(key)
    }

    fn set_full(&mut self, key: K, value: V, origin: u8, signal: Signal) -> bool {
        let mut s = self.lock().unwrap();
        s.set_full(key, value, origin, signal)
    }

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<E>>>) {
        let mut s = self.lock().unwrap();
        s.broadcast(tx);
    }
}

impl<K, V, E: Clone, T: Store<K,V,E>> StoreSetIm<K, V, E> for Arc<Mutex<T>> {
    fn set_full(&self, key: K, value: V, origin: u8, signal: Signal) -> bool {
        let mut s = self.lock().unwrap();
        s.set_full(key, value, origin, signal)
    }

    fn broadcast(&self, tx: Option<broadcast::Sender<Event<E>>>) {
        let mut s = self.lock().unwrap();
        s.broadcast(tx);
    }
}

