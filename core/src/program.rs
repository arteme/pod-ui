use crate::model::Config;
use crate::store::Store;

use log::*;
use crate::raw::Raw;

pub fn load_patch_dump(raw: &mut Raw, page: Option<usize>, data: &[u8], origin: u8) {
    let mut set_value: Box<FnMut(usize, u8)> = if page.is_none() || page.unwrap() == raw.page {
        Box::new(move |i: usize, byte: u8| raw.set(i, byte, origin))
    } else {
        let page = page.unwrap();
        Box::new(move |i: usize, byte: u8| { raw.set_page_value(page, i, byte); })
    };

    for (i, byte) in data.iter().enumerate() {
        set_value(i, *byte);
    }
}

pub fn store_patch_dump_buf(raw: &Raw, page: Option<usize>, config: &Config, data: &mut [u8]) {
    let page = page.unwrap_or(raw.page);
    for i in 0 .. config.program_size {
        raw.get_page_value(page, i)
            .map(|v| data[i] = v)
            .or_else(|| { warn!("No value at position {}", i); None });
    }
}

pub fn store_patch_dump(raw: &Raw, page: Option<usize>, config: &Config) -> Vec<u8> {
    let mut data = vec![0; config.program_size];
    store_patch_dump_buf(raw, page, config, data.as_mut_slice());

    data
}

pub fn load_all_dump(raw: &mut Raw, data: &[u8], config: &Config, origin: u8) {
    let mut chunks = data.chunks(config.program_num);
    for i in 0 .. config.program_num {
        let chunk = chunks.next().unwrap();
        load_patch_dump(raw, Some(i), chunk, origin);
    }
}

pub fn store_all_dump(raw: &Raw, config: &Config) -> Vec<u8> {
    let mut data = vec![0; config.program_size * config.program_num];
    let mut chunks = data.chunks_mut(config.program_num);
    for i in 0 .. config.program_num {
        let chunk = chunks.next().unwrap();
        store_patch_dump_buf(raw, Some(i), config, chunk);
    }

    data
}
