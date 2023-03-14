use std::ops::Range;
use crate::model::Config;

#[derive(Clone)]
pub struct StrEncoder {
    address: Range<usize>
}

impl StrEncoder {
    pub fn new(config: &Config) -> Self {
        let address =
            config.program_name_addr .. config.program_name_addr + config.program_name_length;

        Self { address }
    }

    pub fn str_from_buffer(&self, buffer: &[u8]) -> String {
        let mut vec = vec![0u8; self.address.len()];
        let vec_data = vec.as_mut_slice();
        for i in 0 .. vec_data.len() {
            vec_data[i] = buffer.get(self.address.start + i).cloned().unwrap_or(0);
            // read until the first '\0' character
            if vec_data[i] == 0 {
                break;
            }
        }
        String::from_utf8_lossy(vec_data).to_string()
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{0}')
            .to_string()
            // If we need to remove invalid chars:
            //.chars().map(|c| if is_valid_char(c) { c } else { '_' })
            //.collect()
    }

    pub fn str_to_buffer(&self, str: &str, buffer: &mut [u8]) {
        let str_data = str.as_bytes();
        for i in 0 .. self.address.len() {
            let byte = str_data.get(i).cloned().unwrap_or(0x20);
            buffer[self.address.start + i] = byte;
        }
    }
}