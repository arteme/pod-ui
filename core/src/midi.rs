
use anyhow::{Result, Context};
use crate::util::nibbles_to_u8_vec;

pub struct Channel {}
impl Channel {
    pub const fn num(n: u8) -> u8 { n }
    pub const fn all() -> u8 { 0x7f }
}

#[derive(Clone, Debug)]
pub enum MidiMessage {
    UniversalDeviceInquiry { channel: u8 },
    ProgramPatchDumpRequest { patch: u8 },
    ProgramEditBufferDumpRequest,
    AllProgramsDumpRequest,
}
impl MidiMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            MidiMessage::UniversalDeviceInquiry { channel } =>
                [0xf0, 0x7e, *channel, 0x06, 0x01, 0xf7].to_vec(),
            MidiMessage::ProgramPatchDumpRequest { patch } =>
                [0xf0, 0x00, 0x01, 0x0c, 0x01, 0x00, 0x00, *patch, 0xf7].to_vec(),
            MidiMessage::ProgramEditBufferDumpRequest =>
                [0xf0, 0x00, 0x01, 0x0c, 0x01, 0x00, 0x01, 0xf7].to_vec(),
            MidiMessage::AllProgramsDumpRequest =>
                [0xf0, 0x00, 0x01, 0x0c, 0x01, 0x00, 0x02, 0xf7].to_vec(),
            _ => unimplemented!()
        }
    }
}

#[derive(Clone, Debug)]
pub enum MidiResponse {
    UniversalDeviceInquiry { channel: u8, family: u16, member: u16, ver: String },
    ProgramPatchDump { patch: u8, ver: u8, data: Vec<u8> },
    ProgramEditBufferDump { ver: u8, data: Vec<u8> },
    AllProgramsDump { ver: u8, data: Vec<u8> },

    ControlChange { channel: u8, control: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 }
}
impl MidiResponse {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let len = bytes.len();
        if len < 1 {
            return Err(anyhow!("Zero-size MIDI message"))
        }
        if bytes[0] == 0xf0 {
            // sysex message
            if bytes[1] == 0x7e && bytes.len() == 17 {
                // universal device inquiry response

                assert_eq!(array_ref!(bytes, 5, 3), &[0x00, 0x01, 0x0c]);
                //    .context("Not a Line6 manufacturer id");

                return Ok(MidiResponse::UniversalDeviceInquiry {
                    channel: bytes[2],
                    family: u16::from_le_bytes(array_ref!(bytes, 8, 2).clone()),
                    member: u16::from_le_bytes(array_ref!(bytes, 10, 2).clone()),
                    ver: String::from_utf8(array_ref!(bytes, 12, 4).to_vec())
                        .context("Error converting bytes to UTF-8 string")?
                })
            }

            let id = array_ref!(bytes, 1, 4);
            if id == &[0x00, 0x01, 0x0c, 0x01] {
                // program dump response
                match array_ref!(bytes, 5, 2) {
                    &[0x01, 0x00] => return Ok(MidiResponse::ProgramPatchDump {
                        patch: bytes[7],
                        ver: bytes[8],
                        data: nibbles_to_u8_vec(&bytes[9 .. len-1])
                    }),
                    &[0x01, 0x01] => return Ok(MidiResponse::ProgramEditBufferDump {
                        ver: bytes[7],
                        data: nibbles_to_u8_vec(&bytes[8 .. len-1])
                    }),
                    &[0x01, 0x02] => return Ok(MidiResponse::AllProgramsDump {
                        ver: bytes[7],
                        data: nibbles_to_u8_vec(&bytes[8 .. len-1])
                    }),
                    _ => return Err(anyhow!("Unknown program dump response"))
                }
            }

            return Err(anyhow!("Failed to parse SysEx message!"))
        }
        if (bytes[0] & 0xf0) == 0xb0 {
            // control change
            return Ok(MidiResponse::ProgramChange { channel: bytes[0] & 0x0f, program: bytes[1] })
        }
        if (bytes[0] & 0xf0) == 0xc0 {
            // program change
            return Ok(MidiResponse::ControlChange { channel: bytes[0] & 0x0f, control: bytes[1], value: bytes[2] })
        }

        Err(anyhow!("Failed to parse MIDI message"))
    }
}