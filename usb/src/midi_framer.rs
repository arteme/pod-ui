/// A framer for USB-MIDI protocol, converting MIDI data to USB-MIDI
/// Event Packets and back according to the spec: https://www.usb.org/sites/default/files/midi10.pdf
/// It assumes CN=0 as it does in PocketPOD. In case of new devices
/// where this is different, proper CN handling may need to be implemented
/// later

use log::{error, warn};
use crate::framer::*;

/// A framer for incoming messages using the USB-MIDI protocol, converting
/// USB-MIDI Event Packet stream to MIDI messages
pub struct UsbMidiInFramer {
    sysex_buffer: Vec<u8>,
    sysex_offset: usize
}

impl UsbMidiInFramer {
    fn new() -> Self {
        let sysex_buffer = vec![0u8; 1024];

        Self { sysex_buffer, sysex_offset: 0 }
    }
}

impl InFramer for UsbMidiInFramer {
    fn decode_incoming(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        if bytes.len() % 4 != 0 {
            warn!("Incoming bytes slice size {} is not a multiple of 4", bytes.len())
        }

        let mut ret = vec![];
        bytes.chunks_exact(4).for_each(|b| {
            let mut push_sysex = false;
            let mut sysex_ptr = &mut self.sysex_buffer[self.sysex_offset .. self.sysex_offset + 3];
            match b[0] {
                0x0b => ret.push( b[1 .. 4].to_vec() ),
                0x0c => ret.push( b[1 .. 3].to_vec() ),
                0x04 => {
                    sysex_ptr.copy_from_slice(&b[1 .. b.len()]);
                    self.sysex_offset += 3;
                }
                0x05 ..= 0x07 => {
                    sysex_ptr.copy_from_slice(&b[1 .. b.len()]);
                    self.sysex_offset += match b[0] {
                        0x05 => 1,
                        0x06 => 2,
                        0x07 => 3,
                        _ => unreachable!()
                    };
                    push_sysex = true;
                },
                0x00 => {} // silently drop 0x00-packets
                _ => warn!("Unsupported event packet: {:02x?}", b)
            }
            if push_sysex {
                ret.push(
                    self.sysex_buffer[..self.sysex_offset].to_vec()
                );
                self.sysex_offset = 0;
            }
        });

        ret
    }
}

/// A framer for outgoing messages using the USB-MIDI protocol, converting
/// MIDI messages to USB-MIDI Event Packets
pub struct UsbMidiOutFramer;

impl OutFramer for UsbMidiOutFramer {
    fn encode_outgoing(&self, bytes: &[u8]) -> Vec<Vec<u8>> {
        match bytes[0] {
            0xb0 => vec![
                [ &[0x0b], bytes ].concat()
            ],
            0xc0 => vec![
                [ &[0x0c], bytes, &[0x00] ].concat()
            ],
            0xf0 => {
                bytes.chunks(3).map(|b| {
                    if b.last() == Some(&0xf7) {
                        // sysex finishing
                        match b.len() {
                            3 => [ &[0x07], b ].concat(),
                            2 => [ &[0x06], b, &[0x00] ].concat(),
                            1 => [ &[0x05], b, &[0x00, 0x00] ].concat(),
                            _ => unreachable!()
                        }
                    } else {
                        [ &[0x04], b ].concat()
                    }
                }).collect::<Vec<_>>()
            }
            _ => {
                error!("Unsupported midi message {:?}", bytes);
                vec![]
            }
        }
    }
}

pub fn new_usb_midi_framer() -> (BoxedInFramer, BoxedOutFramer) {
    (Box::new(UsbMidiInFramer::new()), Box::new(UsbMidiOutFramer))
}
