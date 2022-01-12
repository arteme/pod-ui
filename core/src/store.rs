use tokio::sync::broadcast;
use std::sync::{Mutex, Arc};

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
    tx: broadcast::Sender<Event<K>>,
    rx: broadcast::Receiver<Event<K>>
}

impl <K: Clone> StoreBase<K> {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel::<Event<K>>(16);
        StoreBase { tx, rx }
    }

    pub fn send_signal(&self, key: K, value_changed: bool, origin: u8, signal: Signal) -> () {
        let send = match signal {
            Signal::Force => true,
            Signal::Change if value_changed => true,
            _ => false
        };
        if send {
            let event = Event { key, origin, signal };
            self.tx.send(event);
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event<K>> {
        self.tx.subscribe()
    }
}


pub trait Store<K, V, E: Clone> {
    fn has(&self, key: K) -> bool;
    fn get(&self, key: K) -> Option<V>;
    fn set_full(&mut self, key: K, value: V, origin: u8, signal: Signal) -> ();
    fn set(&mut self, key: K, value: V, origin: u8) -> () {
        self.set_full(key, value, origin, Signal::Change)
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<E>>;
}

pub trait StoreSetIm<K, V, E> {
    fn set_full(&self, key: K, value: V, origin: u8, signal: Signal) -> ();
    fn set(&self, key: K, value: V, origin: u8) -> () {
        self.set_full(key, value, origin, Signal::Change)
    }
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

    fn set_full(&mut self, key: K, value: V, origin: u8, signal: Signal) {
        let mut s = self.lock().unwrap();
        s.set_full(key, value, origin, signal);
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<E>> {
        let s = self.lock().unwrap();
        s.subscribe()
    }
}

impl<K, V, E: Clone, T: Store<K,V,E>> StoreSetIm<K, V, E> for Arc<Mutex<T>> {
    fn set_full(&self, key: K, value: V, origin: u8, signal: Signal) {
        let mut s = self.lock().unwrap();
        s.set_full(key, value, origin, signal);
    }
}

