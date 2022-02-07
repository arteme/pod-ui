use crate::store::*;
use log::*;
use tokio::sync::broadcast;

pub struct Strings {
    store: StoreBase<usize>,
    values: Box<[String]>
}

impl Strings {
    pub fn new(size: usize) -> Self {
        let values = vec![String::default(); size].into_boxed_slice();
        Strings { store: StoreBase::new(), values }
    }

}

impl Store<usize, String, usize> for Strings {
    fn has(&self, idx: usize) -> bool {
        idx < self.values.len()
    }

    fn get(&self, idx: usize) -> Option<String> {
        self.values.get(idx).cloned()
    }

    fn set_full(&mut self, idx: usize, val: String, origin: u8, signal: Signal) -> () {
        info!("set {:?} = {:?} <{}>", idx, val, origin);

        let prev = self.values.get(idx);
        if prev.is_none() {
            return;
        }

        let value_changed = prev.unwrap() != &val;
        self.values[idx] = val;

        self.store.send_signal(idx, value_changed, origin, signal);
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<usize>> {
        self.store.subscribe()
    }
}
