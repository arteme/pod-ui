use tokio::sync::broadcast;
use crate::event::Origin;
use crate::model::Config;
use crate::names::ProgramNames;
use crate::store::{Event, Store};

pub struct ProgramsDump {
    program_num: usize,
    program_size: usize,
    data: Box<[u8]>,
    modified: Box<[bool]>,
    names: ProgramNames
}

impl ProgramsDump {
    pub fn new(config: &Config) -> Self {
        let program_num = config.program_num;
        let program_size = config.program_size;
        let data = vec![0u8; program_num * program_size].into_boxed_slice();
        let modified = vec![false; program_num * program_size].into_boxed_slice();
        let names = ProgramNames::new(config);

        Self { program_num, program_size, data, modified, names }
    }

    pub fn broadcast_names(&mut self, tx: Option<broadcast::Sender<Event<usize,String>>>) {
        self.names.broadcast(tx)
    }

    pub fn program_num(&self) -> usize {
        self.program_num
    }


    pub fn program_size(&self) -> usize {
        self.program_size
    }

    #[inline]
    pub fn data(&self, page: usize) -> Option<&[u8]> {
        nth_chunk(&self.data, page, self.program_size)
    }

    pub fn name(&self, page: usize) -> Option<String> {
        self.names.get(page)
    }

    pub fn update_name_from_data(&mut self, page: usize, origin: Origin) {
        let data = nth_chunk(&self.data, page, self.program_size);
        if let Some(data) = data {
            self.names.update_from_data(page, data, origin.into())
        }
    }

    pub fn data_mut(&mut self, page: usize) -> Option<&mut [u8]> {
        nth_chunk_mut(&mut self.data, page, self.program_size)
    }

    pub fn set_name(&mut self, page: usize, name: String, origin: Origin) -> bool {
        self.names.set(page, name, origin.into())
    }

    pub fn modified(&self, page: usize) -> bool {
        self.modified.get(page).unwrap_or(&false).clone()
    }

    pub fn set_modified(&mut self, page: usize, modified: bool) {
        self.modified.get_mut(page).map(|m| *m = modified);
    }

    pub fn set_all_modified(&mut self, modified: bool) {
        self.modified.iter_mut().for_each(|m| *m = modified);
    }
}
fn nth_chunk(data: &[u8], page: usize, page_size: usize) -> Option<&[u8]> {
    data.chunks(page_size).nth(page)
}

fn nth_chunk_mut(data: &mut [u8], page: usize, page_size: usize) -> Option<&mut [u8]> {
    data.chunks_mut(page_size).nth(page)
}
