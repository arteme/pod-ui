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