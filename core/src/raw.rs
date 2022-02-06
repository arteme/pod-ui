use anyhow::*;
use tokio::sync::broadcast;
use log::*;
use crate::config::UNSET;
use crate::store::*;

pub struct Raw {
    store: StoreBase<usize>,
    pub page: usize,
    pub num_pages: usize,
    pub page_size: usize,
    values: Box<[u8]>
}

impl Raw {
    pub fn new(page_size: usize, num_pages: usize) -> Self {
        let size = page_size * num_pages;
        let values = vec![0u8; size].into_boxed_slice();

        Raw { store: StoreBase::new(), page: 0, num_pages, page_size, values }
    }

    pub fn set_page(&mut self, page: usize) -> Result<()> {
        if page > self.num_pages {
            bail!("Page {} out of bounds", page);
        }

        self.page = page;

        Ok(())
    }

    pub fn get_page_value(&self, page: usize, idx: usize) -> Option<u8> {
        if idx > self.page_size || page > self.num_pages {
            return None;
        }
        self.values.get(page * self.page_size + idx).cloned()
    }

    pub fn set_page_value(&mut self, page: usize, idx: usize, val: u8) -> Option<u8> {
        if idx > self.page_size || page > self.num_pages {
            return None;
        }
        let prev = self.values.get(page * self.page_size + idx).cloned();
        self.values[page * self.page_size + idx] = val;

        prev
    }
}

impl Store<usize, u8, usize> for Raw {
    fn has(&self, idx: usize) -> bool {
        idx < self.page_size
    }

    fn get(&self, idx: usize) -> Option<u8> {
        self.get_page_value(self.page, idx)
    }

    fn set_full(&mut self, idx: usize, val: u8, origin: u8, signal: Signal) -> () {
        info!("set {:?} = 0x{:02x} ({}) <{}>", idx, val, val, origin);

        let prev = self.set_page_value(self.page, idx, val);
        if prev.is_none() {
            return;
        }

        let value_changed = prev.unwrap() != val;
        self.store.send_signal(idx, value_changed, origin, signal);
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<usize>> {
        self.store.subscribe()
    }
}