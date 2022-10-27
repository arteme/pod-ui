use anyhow::{Result, Context};
use log::warn;
use crate::util::*;

pub struct Channel {}
impl Channel {
    pub const fn num(n: u8) -> u8 { n }
    pub const fn all() -> u8 { 0x7f }
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    XtEditBufferDumpRequest,
    XtEditBufferDump { id: u8, data: Vec<u8> },
    XtPatchDumpRequest { patch: u16 },
    XtPatchDump { patch: u16, id: u8, data: Vec<u8> },
    XtPatchDumpEnd,

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
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x0e, 0x01, *packs, 0xf7].to_vec(),
            MidiMessage::XtEditBufferDumpRequest =>
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x75, 0xf7].to_vec(),
            MidiMessage::XtEditBufferDump { id,  data } => {
                let mut msg = vec![0xf0, 0x00, 0x01, 0x0c, 0x03, 0x74, *id];
                msg.extend(data);
                msg.extend_from_slice(&[0xf7]);
                msg
            }
            MidiMessage::XtPatchDumpRequest { patch } => {
                let (p1, p2) = u16_to_2_u7(*patch);
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x73, p1, p2, 0x00, 0x00, 0xf7].to_vec()
            }
            MidiMessage::XtPatchDump { patch, id, data } => {
                let (p1, p2) = u16_to_2_u7(*patch);
                let mut msg = vec![0xf0, 0x00, 0x01, 0x0c, 0x03, 0x71, *id, p1, p2];
                msg.extend(data);
                msg.extend_from_slice(&[0xf7]);
                msg
            }
            MidiMessage::XtPatchDumpEnd =>
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x72, 0xf7].to_vec(),

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
            bail!("Zero-size MIDI message")
        }

        return match bytes.as_slice() {
            // sysex message
            [0xf0, ..] => {
                let (canceled, len) = Self::sysex_length(&bytes);
                if canceled {
                    bail!("Sysex message ({} bytes) cancelled", len);
                }
                let bytes = &bytes[1 .. len - 1];

                match bytes {
                    // universal device inquiry
                    [0x7e, c, 0x06, 0x01] => {
                        Ok(MidiMessage::UniversalDeviceInquiry { channel: *c })
                    }
                    // line6-specific universal device inquiry response
                    [0x7e, c, 0x06, 0x02, 0x00, 0x01, 0x0c, payload @ ..] if payload.len() == 8 => {
                        Ok(MidiMessage::UniversalDeviceInquiryResponse {
                            channel: *c,
                            family: u16::from_le_bytes(array_ref!(payload, 0, 2).clone()),
                            member: u16::from_le_bytes(array_ref!(payload, 2, 2).clone()),
                            ver: String::from_utf8(array_ref!(payload, 4, 4).to_vec())
                                .context("Error converting bytes to UTF-8 string")?
                        })
                    }
                    // line6-specific sysex message
                    [0x00, 0x01, 0x0c, payload @ ..] => {
                        match payload {
                            [0x01, 0x00, 0x00, p] => Ok(MidiMessage::ProgramPatchDumpRequest {
                                patch: *p
                            }),
                            [0x01, 0x00, 0x01] => Ok(MidiMessage::ProgramEditBufferDumpRequest {}),
                            [0x01, 0x00, 0x02] => Ok(MidiMessage::AllProgramsDumpRequest {}),
                            [0x01, 0x01, 0x00, p, v, data @ ..] if data.len() >= 2 =>
                                Ok(MidiMessage::ProgramPatchDump {
                                    patch: *p,
                                    ver: *v,
                                    data: nibbles_to_u8_vec(&data)
                                }),
                            [0x01, 0x01, 0x01, v, data @ ..] if data.len() >= 2 =>
                                Ok(MidiMessage::ProgramEditBufferDump {
                                    ver: *v,
                                    data: nibbles_to_u8_vec(&data)
                                }),
                            [0x01, 0x01, 0x02, v, data @ ..] if data.len() >= 2 =>
                                Ok(MidiMessage::AllProgramsDump {
                                    ver: *v,
                                    data: nibbles_to_u8_vec(&data)
                                }),
                            [0x03, 0x75] => Ok(MidiMessage::XtEditBufferDumpRequest),
                            [0x03, 0x74, i, data @ ..] =>
                                Ok(MidiMessage::XtEditBufferDump {
                                    id: *i,
                                    data: data.to_vec()
                                }),
                            [0x03, 0x73, p1, p2, 0x00, 0x00] =>
                                Ok(MidiMessage::XtPatchDumpRequest {
                                    patch: u16_from_2_u7(*p1, *p2)
                                }),
                            [0x03, 0x72] => Ok(MidiMessage::XtPatchDumpEnd),
                            [0x03, 0x71, i, p1, p2, data @ ..] =>
                                Ok(MidiMessage::XtPatchDump {
                                    id: *i,
                                    patch: u16_from_2_u7(*p1, *p2),
                                    data: data.to_vec()
                                }),

                            _ => bail!("Unknown sysex message")
                        }
                    },
                    _ => bail!("Unknown sysex message")
                }
            }
            // control change
            [b0, b1, b2] if b0 & 0xf0 == 0xb0 => {
                Ok(MidiMessage::ControlChange { channel: *b0 & 0x0f, control: *b1, value: *b2 })
            }
            // program change
            [b0, b1] if b0 & 0xf0 == 0xc0 => {
                Ok(MidiMessage::ProgramChange { channel: *b0 & 0x0f, program: *b1 })
            }
            _ => bail!("Unknown MIDI message")
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::midi::MidiMessage;

    #[test]
    fn message_parsing_should_not_crash() {
        let messages: Vec<MidiMessage> = vec![
            MidiMessage::UniversalDeviceInquiry { channel: 1 },
            MidiMessage::UniversalDeviceInquiryResponse { channel: 1, family: 1, member: 1, ver: "0304".into() },
            MidiMessage::AllProgramsDumpRequest,
            MidiMessage::AllProgramsDump { ver: 0, data: vec![1] },
            MidiMessage::ProgramPatchDumpRequest { patch: 7 },
            MidiMessage::ProgramPatchDump { patch: 7, ver: 0, data: vec![1] },
            MidiMessage::ProgramEditBufferDumpRequest,
            MidiMessage::ProgramEditBufferDump { ver: 0, data: vec![1] },
            MidiMessage::ControlChange { channel: 2, control: 64, value: 127 },
            MidiMessage::ProgramChange { channel: 3, program: 32 }
        ];

        for msg in messages.iter() {
            let bytes = msg.to_bytes();
            println!("{:?}", msg);
            println!("{:x?} len={}", bytes, bytes.len());

            let is_sysex = bytes[0] == 0xf0;
            // we add an extra terminator character to test malformed sysex parsing,
            // so the run-to length in case of sysex is shorter
            let run_to_len = if is_sysex { bytes.len() - 1 } else { bytes.len() };
            for i in 1 ..= run_to_len {
                let mut part = bytes[0 .. i].to_vec();

                // terminate a sysex message
                if is_sysex { part.push(0xf7) }
                
                let result = MidiMessage::from_bytes(part);
                print!("{:?} ", result);
                let result = result.ok();
                if i < run_to_len {
                    println!("neq?");
                    assert!(result.is_none());
                } else {
                    println!("eq?");
                    assert_eq!(result.as_ref(), Some(msg));
                }
            }
        }
    }
}

