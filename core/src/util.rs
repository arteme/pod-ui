use log::*;

pub fn nibble_to_u8(bytes: &[u8; 2]) -> u8 {
    (bytes[0] << 4) | (bytes[1] & 0x0f)
}

pub fn nibbles_to_u8_vec(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() & 1 == 1 {
        error!("nibbles_to_u8_vec got a slice of odd size {}", bytes.len());
    }

    let len = bytes.len() / 2;
    let mut arr: Vec<u8> = vec![0; len];

    for (i, v) in arr.iter_mut().enumerate() {
        *v = nibble_to_u8(array_ref![bytes, i*2, 2]);
    }

    arr
}

pub fn u8_to_nibble(byte: u8, buf: &mut [u8; 2]) {
    buf[0] = (byte >> 4) & 0x0f;
    buf[1] = byte & 0x0f;
}

pub fn u8_to_nibbles_vec(bytes: &[u8]) -> Vec<u8> {
    let len = bytes.len() * 2;
    let mut arr: Vec<u8> = vec![0; len];

    for (i, v) in bytes.iter().enumerate() {
        u8_to_nibble(*v, array_mut_ref![arr, i*2, 2]);
    }

    arr
}

pub trait OptionToResultsExt {
    type In;
    fn and_then_r<U, E, F: FnOnce(Self::In) -> Result<Option<U>, E>>(self, f: F) -> Result<Option<U>, E>;
}

impl<T> OptionToResultsExt for Option<T> {
    type In = T;
    fn and_then_r<U, E, F: FnOnce(Self::In) -> Result<Option<U>, E>>(self, f: F) -> Result<Option<U>, E> {
        match self {
            Some(x) => f(x),
            None => Ok(None)
        }
    }
}

pub fn u16_to_2_u7(v: u16) -> (u8, u8) {
    let b1 = v >> 7;
    let b2 = v & 0x7f;
    (b1 as u8, b2 as u8)
}

pub fn u16_from_2_u7(v1: u8, v2: u8) -> u16 {
    (v1 as u16) << 7 | (v2 as u16)
}

pub fn u16_to_4_u4(v: u16) -> (u8, u8, u8, u8) {
    let b1 = (v >> 12) & 0x0f;
    let b2 = (v >> 8) & 0x0f;
    let b3 = (v >> 4) & 0x0f;
    let b4 = v & 0x0f;
    (b1 as u8, b2 as u8, b3 as u8, b4 as u8)
}

pub fn u16_from_4_u4(v1: u8, v2: u8, v3: u8, v4: u8) -> u16 {
    (v1 as u16) << 12 | (v2 as u16) << 8 | (v3 as u16) << 4 | (v4 as u16)
}

/// A shorthand for `Default::default()` while waiting on
pub fn def<T: Default>() -> T {
    Default::default()
}