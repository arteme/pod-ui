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
    XtBufferDump { id: u8, data: Vec<u8> },
    XtPatchDumpRequest { patch: u16 },
    XtPatchDump { patch: u16, id: u8, data: Vec<u8> },
    XtPatchDumpEnd,
    XtSaved { patch: u16 },
    XtStoreStatus { success: bool },
    XtTunerNoteRequest,
    XtTunerNote { note: u16 },
    XtTunerOffsetRequest,
    XtTunerOffset { offset: u16 },
    XtProgramNumberRequest,
    XtProgramNumber { program: u16 },

    ControlChange { channel: u8, control: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 }
}

pub struct PodXtPatch;
impl PodXtPatch {
    pub fn to_midi(value: u16) -> u16 {
        let bank = (value >> 8) & 0xff;
        let patch = value & 0xff;
        match (bank, patch) {
            (0, 0 ..= 63) => patch,
            (0, 64 ..= 127) => patch + 128,
            (1, 0 ..= 63) => patch + 64,
            (1, 64 ..= 127) => patch + 192,
            (2, 0 ..= 63) => patch + 128,
            (2, 64 ..= 127) => patch + 256,
            _ => panic!("unsupported patch_to_midi value: {}", value)
        }
    }

    pub fn from_midi(value: u16) -> u16 {
        let (bank, patch) = match value {
            0 ..= 63 => (0, value),
            192 ..= 255 => (0, value - 128),
            64 ..= 127 => (1, value - 64),
            256 ..= 319 => (1, value - 192),
            128 ..= 191 => (2, value - 128),
            320 ..= 383 => (2, value - 256),
            _ => panic!("unsupported patch_from_midi value: {}", value)
        };
        (bank << 8) | patch
    }
}

pub struct PodXtSaved;
impl PodXtSaved {
    pub fn to_midi(value: u16) -> u16 {
        let bank = ((value >> 8) & 0xff) as u8;
        let patch = (value & 0xff) as u8;

        if bank > 2 {
            panic!("unsupported saved_to_midi value: {}", value);
        }

        u16_from_2_u7(bank + 1, patch)
    }

    pub fn from_midi(value: u16) -> u16 {
        let (bank, patch) = u16_to_2_u7(value);

        if bank == 0 || bank > 3 {
            panic!("unsupported saved_from_midi value: {}", value)
        }

        ((bank as u16 - 1) << 8) | patch as u16
    }
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
            MidiMessage::XtBufferDump { id,  data } => {
                let mut msg = vec![0xf0, 0x00, 0x01, 0x0c, 0x03, 0x74, *id];
                msg.extend(data);
                msg.extend_from_slice(&[0xf7]);
                msg
            }
            MidiMessage::XtPatchDumpRequest { patch } => {
                let patch = PodXtPatch::to_midi(*patch);
                let (p1, p2) = u16_to_2_u7(patch);
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x73, p1, p2, 0x00, 0x00, 0xf7].to_vec()
            }
            MidiMessage::XtPatchDump { patch, id, data } => {
                let patch = PodXtPatch::to_midi(*patch);
                let (p1, p2) = u16_to_2_u7(patch);
                let mut msg = vec![0xf0, 0x00, 0x01, 0x0c, 0x03, 0x71, *id, p1, p2];
                msg.extend(data);
                msg.extend_from_slice(&[0xf7]);
                msg
            }
            MidiMessage::XtPatchDumpEnd =>
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x72, 0xf7].to_vec(),
            MidiMessage::XtSaved { patch} => {
                let patch = PodXtSaved::to_midi(*patch);
                let (p1, p2) = u16_to_2_u7(patch);
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x24, p1, p2, 0xf7].to_vec()
            },
            MidiMessage::XtStoreStatus { success } => {
                let f = !success as u8;
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x50 | f, 0xf7].to_vec()
            }
            MidiMessage::XtTunerNoteRequest => {
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x57, 0x16, 0xf7].to_vec()
            }
            MidiMessage::XtTunerNote { note } => {
                let (p1, p2, p3, p4) = u16_to_4_u4(*note);
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x56, 0x16, p1, p2, p3, p4, 0xf7].to_vec()
            }
            MidiMessage::XtTunerOffsetRequest => {
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x57, 0x17, 0xf7].to_vec()
            }
            MidiMessage::XtTunerOffset { offset } => {
                let (p1, p2, p3, p4) = u16_to_4_u4(*offset);
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x56, 0x17, p1, p2, p3, p4, 0xf7].to_vec()
            }
            MidiMessage::XtProgramNumberRequest => {
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x57, 0x11, 0xf7].to_vec()
            }
            MidiMessage::XtProgramNumber { program } => {
                let (p1, p2, p3, p4) = u16_to_4_u4(*program);
                [0xf0, 0x00, 0x01, 0x0c, 0x03, 0x56, 0x11, p1, p2, p3, p4, 0xf7].to_vec()
            }

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
        if bytes.len() < 1 {
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
                            [0x03, 0x0e, 0x00] =>
                                Ok(MidiMessage::XtInstalledPacksRequest),
                            [0x03, 0x0e, 0x01, p] =>
                                Ok(MidiMessage::XtInstalledPacks { packs: *p }),
                            [0x03, 0x75] => Ok(MidiMessage::XtEditBufferDumpRequest),
                            [0x03, 0x74, i, data @ ..] =>
                                Ok(MidiMessage::XtBufferDump {
                                    id: *i,
                                    data: data.to_vec()
                                }),
                            [0x03, 0x73, p1, p2, 0x00, 0x00] => {
                                let patch = u16_from_2_u7(*p1, *p2);
                                let patch = PodXtPatch::from_midi(patch);
                                Ok(MidiMessage::XtPatchDumpRequest { patch })
                            }
                            [0x03, 0x72] => Ok(MidiMessage::XtPatchDumpEnd),
                            [0x03, 0x71, i, p1, p2, data @ ..] => {
                                let patch = u16_from_2_u7(*p1, *p2);
                                let patch = PodXtPatch::from_midi(patch);
                                Ok(MidiMessage::XtPatchDump {
                                    id: *i,
                                    patch,
                                    data: data.to_vec()
                                })
                            }
                            [0x03, 0x24, p1, p2] => {
                                let patch = u16_from_2_u7(*p1, *p2);
                                let patch = PodXtSaved::from_midi(patch);
                                Ok(MidiMessage::XtSaved { patch })
                            }
                            [0x03, 0x50] => Ok(MidiMessage::XtStoreStatus { success: true }),
                            [0x03, 0x51] => Ok(MidiMessage::XtStoreStatus { success: false }),
                            [0x03, 0x57, 0x16] => Ok(MidiMessage::XtTunerNoteRequest),
                            [0x03, 0x57, 0x17] => Ok(MidiMessage::XtTunerOffsetRequest),
                            [0x03, 0x56, 0x16, p1, p2, p3, p4] => {
                                let note = u16_from_4_u4(*p1, *p2, *p3, *p4);
                                Ok(MidiMessage::XtTunerNote { note })
                            }
                            [0x03, 0x56, 0x17, p1, p2, p3, p4] => {
                                let offset = u16_from_4_u4(*p1, *p2, *p3, *p4);
                                Ok(MidiMessage::XtTunerOffset { offset })
                            }
                            [0x03, 0x57, 0x11] => Ok(MidiMessage::XtProgramNumberRequest),
                            [0x03, 0x56, 0x11, p1, p2, p3, p4] => {
                                let program = u16_from_4_u4(*p1, *p2, *p3, *p4);
                                Ok(MidiMessage::XtProgramNumber { program })
                            }


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

