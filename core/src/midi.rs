
use crate::midi::MidiMessage::UniversalDeviceInquiry;
use anyhow::{Result, Context};

pub struct Channel {}
impl Channel {
    pub const fn num(n: u8) -> u8 { n }
    pub const fn all() -> u8 { 0x7f }
}

#[derive(Clone, Debug)]
pub enum MidiMessage {
    UniversalDeviceInquiry { channel: u8 }
}
impl MidiMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            UniversalDeviceInquiry { channel } => [0xf0, 0x7e, *channel, 0x06, 0x01, 0xf7]
        }.to_vec()
    }
}

#[derive(Clone, Debug)]
pub enum MidiResponse {
    UniversalDeviceInquiry { channel: u8, family: u16, member: u16, ver: String }
}
impl MidiResponse {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        match (bytes.get(0), bytes.len()) {
            (Some(0xf0), 17) => {
                // sysex

                assert_eq!(array_ref!(bytes, 5, 3), &[0x00, 0x01, 0x0c]);
                //    .context("Not a Line6 manufacturer id");

                Ok(MidiResponse::UniversalDeviceInquiry {
                    channel: bytes[2],
                    family: u16::from_le_bytes(array_ref!(bytes, 8, 2).clone()),
                    member: u16::from_le_bytes(array_ref!(bytes, 10, 2).clone()),
                    ver: String::from_utf8(array_ref!(bytes, 12, 4).to_vec())
                        .context("Error converting bytes to UTF-8 string")?
                })


            }
            _ => Err(anyhow!("Failed to parse message"))
        }
    }
}