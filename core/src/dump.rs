use tokio::sync::broadcast::Receiver;
use crate::model::Config;
use crate::names::ProgramNames;
use crate::store::{Event, Store};

pub struct ProgramsDump {
    program_num: usize,
    program_size: usize,
    data: Box<[u8]>,
    names: ProgramNames
}

impl ProgramsDump {
    pub fn new(config: &Config) -> Self {
        let program_num = config.program_num;
        let program_size = config.program_size;
        let data = vec![0u8; program_num * program_size].into_boxed_slice();
        let names = ProgramNames::new(config);

        Self { program_num, program_size, data, names }
    }

    #[inline]
    pub fn data(&self, page: usize) -> Option<&[u8]> {
        nth_chunk(&self.data, page, self.program_size)
    }

    pub fn data_for_program(&self, program: usize) -> Option<&[u8]> {
        // programs as 1-indexed
        self.data(program - 1)
    }

    pub fn name(&self, page: usize) -> Option<String> {
        self.names.get(page)
    }

    pub fn name_for_program(&self, program: usize) -> Option<String> {
        // programs as 1-indexed
        self.name(program - 1)
    }

    pub fn update_name_from_data(&mut self, page: usize, origin: u8) {
        let data = nth_chunk(&self.data, page, self.program_size);
        if let Some(data) = data {
            self.names.str_from_data(page, data, origin)
        }
    }

    pub fn subscribe_to_name_updates(&self) -> Receiver<Event<usize>> {
        self.names.subscribe()

    }

    pub fn data_mut(&mut self, page: usize) -> Option<&mut [u8]> {
        nth_chunk_mut(&mut self.data, page, self.program_size)
    }

    pub fn set_name(&mut self, page: usize, name: String, origin: u8) -> bool {
        self.names.set(page, name, origin)
    }
}
fn nth_chunk(data: &[u8], page: usize, page_size: usize) -> Option<&[u8]> {
    data.chunks(page_size).nth(page)
}

fn nth_chunk_mut(data: &mut [u8], page: usize, page_size: usize) -> Option<&mut [u8]> {
    data.chunks_mut(page_size).nth(page)
}
