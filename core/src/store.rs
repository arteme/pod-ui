use tokio::sync::broadcast;
use std::sync::{Mutex, Arc};

pub trait Store<K, V, S> {
    fn has(&self, key: K) -> bool;
    fn get(&self, key: K) -> Option<V>;
    fn set(&mut self, key: K, value: V, origin: u8) -> ();

    fn subscribe(&self) -> broadcast::Receiver<S>;
}

pub trait StoreSetIm<K, V, S> {
    fn set(&self, key: K, value: V, origin: u8) -> ();
}

impl<K, V, S, T: Store<K,V,S>> Store<K, V, S> for Arc<Mutex<T>> {
    fn has(&self, key: K) -> bool {
        let s = self.lock().unwrap();
        s.has(key)
    }

    fn get(&self, key: K) -> Option<V> {
        let s = self.lock().unwrap();
        s.get(key)
    }

    fn set(&mut self, key: K, value: V, origin: u8) {
        let mut s = self.lock().unwrap();
        s.set(key, value, origin);
    }

    fn subscribe(&self) -> broadcast::Receiver<S> {
        let s = self.lock().unwrap();
        s.subscribe()
    }
}

impl<K, V, S,  T: Store<K,V,S>> StoreSetIm<K, V, S> for Arc<Mutex<T>> {
    fn set(&self, key: K, value: V, origin: u8) {
        let mut s = self.lock().unwrap();
        s.set(key, value, origin);
    }
}
