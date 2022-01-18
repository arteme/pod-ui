
use tokio::sync::broadcast;
use log::*;
use crate::store::*;

pub struct Raw {
    store: StoreBase<usize>,
    values: Box<[u8]>
}

impl Raw {
    pub fn new(size: usize) -> Self {
        let values = vec![0u8; size].into_boxed_slice();

        Raw { store: StoreBase::new(), values }
    }
}

impl Store<usize, u8, usize> for Raw {
    fn subscribe(&self) -> broadcast::Receiver<Event<usize>> {
        self.store.subscribe()
    }

    fn has(&self, idx: usize) -> bool {
        idx < self.values.len()
    }

    fn get(&self, idx: usize) -> Option<u8> {
        self.values.get(idx).cloned()
    }

    fn set_full(&mut self, idx: usize, val: u8, origin: u8, signal: Signal) -> () {
        info!("set {:?} = 0x{:02x} ({}) <{}>", idx, val, val, origin);
        if idx >= self.values.len() {
            return;
        }

        let old = self.values[idx];
        let value_changed = old != val;

        if value_changed {
            self.values[idx] = val;
        }


        self.store.send_signal(idx, value_changed, origin, signal);
    }

}