use bytes::{Bytes, BytesMut};
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

/// A shorthand for `Default::default()` while waiting on
pub fn def<T: Default>() -> T {
    Default::default()
}


pub trait ToBytes {
    fn to_bytes(&self) -> Bytes;
    fn to_bytes_mut(&self) -> BytesMut;
}

impl<const N: usize> ToBytes for [u8; N] {
    fn to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self.as_slice())
    }

    fn to_bytes_mut(&self) -> BytesMut {
        let mut b = BytesMut::with_capacity(self.len());
        b.extend_from_slice(self.as_slice());
        b
    }
}

impl ToBytes for [u8] {
    fn to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(&self)
    }

    fn to_bytes_mut(&self) -> BytesMut {
        let mut b = BytesMut::with_capacity(self.len());
        b.extend_from_slice(&self);
        b
    }
}
