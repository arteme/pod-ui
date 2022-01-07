
use tokio::sync::broadcast;
use log::*;
use crate::store::Store;

pub struct Raw {
    values: Box<[u8]>,

    tx: broadcast::Sender<(usize, u8)>,
    rx: broadcast::Receiver<(usize, u8)>
}

impl Raw {
    pub fn new(size: usize) -> Self {
        let values = vec![0u8; size].into_boxed_slice();
        let (tx, rx) = broadcast::channel::<(usize, u8)>(16);

        Raw { values, tx, rx }
    }
}

impl Store<usize, u8, (usize, u8)> for Raw {
    fn subscribe(&self) -> broadcast::Receiver<(usize, u8)> {
        self.tx.subscribe()
    }

    fn has(&self, idx: usize) -> bool {
        idx < self.values.len()
    }

    fn get(&self, idx: usize) -> Option<u8> {
        self.values.get(idx).cloned()
    }

    fn set(&mut self, idx: usize, val: u8, origin: u8) -> () {
        info!("set {:?} = 0x{:02x} ({}) <{}>", idx, val, val, origin);
        if idx >= self.values.len() {
            return;
        }

        let old = self.values[idx];
        if old != val {
            self.values[idx] = val;
            self.tx.send((idx,origin));
        }
    }

}