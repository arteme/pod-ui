use anyhow::{Result, Context};
use log::warn;
use crate::util::*;

pub struct Channel {}
impl Channel {
    pub const fn num(n: u8) -> u8 { n }
    pub const fn all() -> u8 { 0x7f }
}

#[derive(Clone, Debug)]
pub enum MidiMessage {
    UniversalDeviceInquiry { channel: u8 },
    UniversalDeviceInquiryResponse { channel: u8, family: u16, member: u16, ver: String },

    ProgramPatchDumpRequest { patch: u8 },
    ProgramPatchDump { patch: u8, ver: u8, data: Vec<u8> },
    ProgramEditBufferDumpRequest,
    ProgramEditBufferDump { ver: u8, data: Vec<u8> },
    AllProgramsDumpRequest,
    AllProgramsDump { ver: u8, data: Vec<u8> },

    XtInstalledPacksRequest,
    XtInstalledPacks { packs: u8 },

    ControlChange { channel: u8, control: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 }
}
impl MidiMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            MidiMessage::UniversalDeviceInquiry { channel } =>
                [0xf0, 0x7e, *channel, 0x06, 0x01, 0xf7].to_vec(),
            MidiMessage::UniversalDeviceInquiryResponse { channel, family, member, ver } => {
                let family = u16::to_le_bytes(*family);
                let member = u16::to_le_bytes(*member);
                let ver = format!("{:4}", ver).into_bytes();
                [0xf0, 0x7e, *channel, 0x06, 0x02, 0x00, 0x01, 0x0c, family[0], family[1], member[0], member[1],
                 ver[0], ver[1], ver[2], ver[3], 0xf7].to_vec()
            },
            MidiMessage::ProgramPatchDumpRequest { patch } =>
                [0xf0, 0x00, 0x01, 0x0c, 0x01, 0x00, 0x00, *patch, 0xf7].to_vec(),
            MidiMessage::ProgramPatchDump { patch, ver, data } => {
                let data = u8_to_nibbles_vec(data.as_slice());
                let mut msg = vec![0xf0, 0x00, 0x01, 0x0c, 0x01, 0x01, 0x00, *patch, *ver];
                msg.extend(data);
                msg.extend_from_slice(&[0xf7]);
                msg
            },
            MidiMessage::ProgramEditBufferDumpRequest =>
                [0xf0, 0x00, 0x01, 0x0c, 0x01, 0x00, 0x01, 0xf7].to_vec(),
            MidiMessage::ProgramEditBufferDump { ver, data } => {
                let data = u8_to_nibbles_vec(data.as_slice());
                let mut msg = vec![0xf0, 0x00, 0x01, 0x0c, 0x01, 0x01, 0x01, *ver];
                msg.extend(data);
                msg.extend_from_slice(&[0xf7]);
                msg
            },
            MidiMessage::AllProgramsDumpRequest =>
                [0xf0, 0x00, 0x01, 0x0c, 0x01, 0x00, 0x02, 0xf7].to_vec(),
            MidiMessage::AllProgramsDump { ver, data } => {
                let data = u8_to_nibbles_vec(data.as_slice());
                let mut msg = vec![0xf0, 0x00, 0x01, 0x0c, 0x01, 0x01, 0x02, *ver];
                msg.extend(data);
                msg.extend_from_slice(&[0xf7]);
                msg
            },

            MidiMessage::XtInstalledPacksRequest =>
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x0e, 0x00, 0xf7].to_vec(),
            MidiMessage::XtInstalledPacks { packs } =>
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x0e, 0x00, *packs, 0xf7].to_vec(),

            MidiMessage::ControlChange { channel, control, value } =>
                [0xb0 | *channel & 0x0f, *control, *value].to_vec(),
            MidiMessage::ProgramChange { channel, program } =>
                [0xc0 | *channel & 0x0f, *program].to_vec(),
        }
    }

    fn sysex_length(bytes: &Vec<u8>) -> (bool, usize) {
        let mut canceled = true;
        let mut len = bytes.len();
        if len == 0 || bytes[0] != 0xf0 {
            return (canceled, len);
        }

        let old_len = len;
        for (i, b) in bytes.iter().enumerate().skip(1) {
            if (*b & 0x80) != 0x80 { continue; }

            // a byte with MSB set that is not a sysex terminator = cancel
            len = i + 1;
            canceled = *b != 0xf7;
            break;
        }

        if len != old_len {
            warn!("Correcting sysex message length {} -> {}", old_len, len);
        }

        return (canceled, len);
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let mut len = bytes.len();
        if len < 1 {
            return Err(anyhow!("Zero-size MIDI message"))
        }

        if bytes[0] == 0xf0 {
            // sysex message
            let (canceled, len) = Self::sysex_length(&bytes);
            if canceled {
                return Err(anyhow!("Sysex message ({} bytes) cancelled", len));
            }

            if bytes[1] == 0x7e && len == 17 {
                // universal device inquiry response

                let id = array_ref!(bytes, 5, 3);
                if id != &[0x00, 0x01, 0x0c] {
                    bail!("Not a Line6 manufacturer id: {:x?}", id);
                }

                return Ok(MidiMessage::UniversalDeviceInquiryResponse {
                    channel: bytes[2],
                    family: u16::from_le_bytes(array_ref!(bytes, 8, 2).clone()),
                    member: u16::from_le_bytes(array_ref!(bytes, 10, 2).clone()),
                    ver: String::from_utf8(array_ref!(bytes, 12, 4).to_vec())
                        .context("Error converting bytes to UTF-8 string")?
                })
            }
            if bytes[1] == 0x7e && len == 6 && array_ref!(bytes, 3, 2) == &[0x06, 0x01] {
                // universal device inquiry
                return Ok(MidiMessage::UniversalDeviceInquiry {
                    channel: bytes[2],
                })
            }

            let id = array_ref!(bytes, 1, 3);
            if id == &[0x00, 0x01, 0x0c] {
                // program dump response
                match array_ref!(bytes, 4, 3) {
                    &[0x01, 0x00, 0x00] => return Ok(MidiMessage::ProgramPatchDumpRequest {
                       patch: bytes[7]
                    }),
                    &[0x01, 0x00, 0x01] => return Ok(MidiMessage::ProgramEditBufferDumpRequest {}),
                    &[0x01, 0x00, 0x02] => return Ok(MidiMessage::AllProgramsDumpRequest {}),
                    &[0x01, 0x01, 0x00] => return Ok(MidiMessage::ProgramPatchDump {
                        patch: bytes[7],
                        ver: bytes[8],
                        data: nibbles_to_u8_vec(&bytes[9 .. len-1])
                    }),
                    &[0x01, 0x01, 0x01] => return Ok(MidiMessage::ProgramEditBufferDump {
                        ver: bytes[7],
                        data: nibbles_to_u8_vec(&bytes[8 .. len-1])
                    }),
                    &[0x01, 0x01, 0x02] => return Ok(MidiMessage::AllProgramsDump {
                        ver: bytes[7],
                        data: nibbles_to_u8_vec(&bytes[8 .. len-1])
                    }),
                    &[0x03, 0x0e, 0x00] => return Ok(MidiMessage::XtInstalledPacksRequest),
                    // TODO: XtInstalledPacks

                    _ => return Err(anyhow!("Unknown program dump message"))
                }
            }

            return Err(anyhow!("Failed to parse SysEx message!"))
        }
        if (bytes[0] & 0xf0) == 0xb0 {
            // control change
            return Ok(MidiMessage::ControlChange { channel: bytes[0] & 0x0f, control: bytes[1], value: bytes[2] })
        }
        if (bytes[0] & 0xf0) == 0xc0 {
            // program change
            return Ok(MidiMessage::ProgramChange { channel: bytes[0] & 0x0f, program: bytes[1] })
        }

        Err(anyhow!("Failed to parse MIDI message"))
    }
}