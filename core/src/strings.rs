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

    fn set_full(&mut self, idx: usize, val: String, origin: Origin, signal: Signal) -> bool {
        info!("set {:?} = {:?} <{:?}>", idx, val, origin);

        let prev = self.values.get(idx);
        if prev.is_none() {
            return false;
        }

        let value_changed = prev.unwrap() != &val;
        self.values[idx] = val;

        self.store.send_signal(idx, value_changed, origin, signal);
        value_changed
    }

    fn broadcast(&mut self, tx: Option<broadcast::Sender<Event<usize>>>) {
        self.store.broadcast(tx)
    }
}
