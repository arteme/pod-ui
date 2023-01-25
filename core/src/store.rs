use tokio::sync::broadcast;
use std::sync::{Mutex, Arc};
use log::warn;

#[derive(Clone, PartialEq)]
pub enum Signal {
    None,
    Change,
    Force
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Origin {
    NONE,
    MIDI,
    UI,
}

#[derive(Clone)]
pub struct Event<K, V> {
    pub key: K,
    pub value: V,
    pub origin: Origin,
    pub signal: Signal
}

pub struct StoreBase<K, V> {
    tx: Option<broadcast::Sender<Event<K,V>>>
}

impl <K, V> StoreBase<K,V> {
    pub fn new() -> Self {
        StoreBase { tx: None }
    }

    pub fn send_signal(&self, key: K, value: V, value_changed: bool, origin: Origin, signal: Signal) -> () {
        let send = match signal {
            Signal::Force => true,
            Signal::Change if value_changed => true,
            _ => false
        };
        if send {
            let event = Event { key, value, origin, signal };
            if let Some(tx) = &self.tx {
                tx.send(event)
                    .map_err(|_| warn!("Failed to store event signal"))
                    .unwrap_or_default();
            }
        }
    }

    pub fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<K,V>>>) {
        self.tx = tx;
    }
}


pub trait Store<K, V, E> {
    fn has(&self, key: K) -> bool;
    fn get(&self, key: K) -> Option<V>;
    fn set_full(&mut self, key: K, value: V, origin: Origin, signal: Signal) -> bool;
    fn set(&mut self, key: K, value: V, origin: Origin) -> bool {
        self.set_full(key, value, origin, Signal::Change)
    }

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<E,V>>>);
}

pub trait StoreSetIm<K, V, E> {
    fn set_full(&self, key: K, value: V, origin: Origin, signal: Signal) -> bool;
    fn set(&self, key: K, value: V, origin: Origin) -> bool {
        self.set_full(key, value, origin, Signal::Change)
    }

    fn broadcast(&self, tx: Option<broadcast::Sender<Event<E,V>>>);
}

impl<K, V, E, T: Store<K,V,E>> Store<K, V, E> for Arc<Mutex<T>> {
    fn has(&self, key: K) -> bool {
        let s = self.lock().unwrap();
        s.has(key)
    }

    fn get(&self, key: K) -> Option<V> {
        let s = self.lock().unwrap();
        s.get(key)
    }

    fn set_full(&mut self, key: K, value: V, origin: Origin, signal: Signal) -> bool {
        let mut s = self.lock().unwrap();
        s.set_full(key, value, origin, signal)
    }

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<E,V>>>) {
        let mut s = self.lock().unwrap();
        s.broadcast(tx);
    }
}

impl<K, V, E, T: Store<K,V,E>> StoreSetIm<K, V, E> for Arc<Mutex<T>> {
    fn set_full(&self, key: K, value: V, origin: Origin, signal: Signal) -> bool {
        let mut s = self.lock().unwrap();
        s.set_full(key, value, origin, signal)
    }

    fn broadcast(&self, tx: Option<broadcast::Sender<Event<E,V>>>) {
        let mut s = self.lock().unwrap();
        s.broadcast(tx);
    }
}

