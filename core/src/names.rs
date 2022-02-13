use std::ffi::CStr;
use std::ops::Range;
use tokio::sync::broadcast;
use crate::model::Config;
use crate::raw::Raw;
use crate::store::{Event, Signal, Store};
use crate::strings::Strings;

pub struct ProgramNames {
    names: Strings,
    name_address: Range<usize>
}

impl ProgramNames {
    pub fn new(config: &Config) -> Self {
        let names = Strings::new(config.program_num);
        let name_address =
            config.program_name_addr .. config.program_name_addr + config.program_name_length;

        ProgramNames { names, name_address }
    }

    pub fn str_from_data(&mut self, page: usize, data: &[u8], origin: u8) {
        let mut vec = vec![0u8; self.name_address.len()];
        let mut vec_data = vec.as_mut_slice();
        for i in 0 .. vec_data.len() {
            vec_data[i] = data.get(self.name_address.start + i).cloned().unwrap_or(0);
        }
        let str = String::from_utf8_lossy(vec_data).to_string()
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{0}')
            .to_string();
        self.names.set(page, str, origin);
    }

    pub fn str_to_data(&self, data: &mut [u8], page: usize) {
        let str = self.names.get(page).unwrap();
        let str_data = str.as_bytes();
        for i in 0 .. self.name_address.len() {
            let byte = str_data.get(i).cloned().unwrap_or(0x20);
            data[self.name_address.start + i] = byte;
        }
    }
}

impl Store<usize, String, usize> for ProgramNames {
    fn has(&self, idx: usize) -> bool {
        self.names.has(idx)
    }

    fn get(&self, idx: usize) -> Option<String> {
        self.names.get(idx)
    }

    fn set_full(&mut self, idx: usize, val: String, origin: u8, signal: Signal) -> bool {
        self.names.set_full(idx, val, origin, signal)
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<usize>> {
        self.names.subscribe()
    }
}
