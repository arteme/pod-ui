use crate::model::Config;
use crate::store::Store;

use log::*;
use crate::raw::Raw;

pub fn load_dump(raw: &mut Raw, data: &[u8], origin: u8) {
    for (i, byte) in data.iter().enumerate() {
        raw.set(i, *byte, origin);
    }
}

pub fn dump(raw: &Raw, config: &Config) -> Vec<u8> {
    let mut data = vec![0; config.program_size];
    for i in 0 .. config.program_size {
        raw.get(i)
            .map(|v| data[i] = v)
            .or_else(|| { warn!("No value at position {}", i); None });
    }

    data
}
