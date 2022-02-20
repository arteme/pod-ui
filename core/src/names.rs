use tokio::sync::broadcast;
use crate::model::Config;
use crate::store::{Event, Signal, Store};
use crate::str_encoder::StrEncoder;
use crate::strings::Strings;

pub struct ProgramNames {
    names: Strings,
    encoder: StrEncoder
}

impl ProgramNames {
    pub fn new(config: &Config) -> Self {
        let names = Strings::new(config.program_num);
        let encoder = StrEncoder::new(&config);

        Self { names, encoder }
    }

    pub fn update_from_data(&mut self, page: usize, data: &[u8], origin: u8) {
        let str = self.encoder.str_from_buffer(data);
        self.names.set(page, str, origin);
    }

    pub fn update_to_data(&self, data: &mut [u8], page: usize) {
        let str = self.names.get(page).unwrap();
        self.encoder.str_to_buffer(&str, data);
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
