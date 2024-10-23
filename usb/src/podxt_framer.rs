/// A framer for PODxt USB MIDI, which is essentially MIDI messages without
/// any framing, but with a few quirks (0xb2/0xc2/0xf2)

use crate::framer::*;

/// A framer for incoming messages using the PODxt USB MIDI protocol,
/// which buffers the message data, splitting SysEx messages from the
///incoming data (may be many SysEx messages in one USB transfer) into
/// separate MIDI messages.
/// TODO: check to make sure that 0xb0 and 0xc0 messages are not bundled
///       up into one USB transfer like the SysEx
pub struct PodXtInFramer {
    read_buffer: Vec<u8>,
    read_offset: usize
}

impl PodXtInFramer {
    pub fn new() -> Self {
        let read_buffer = vec![0u8; 1024];

        Self { read_buffer, read_offset: 0 }
    }

    fn find_message(read_ptr: &mut [u8]) -> &[u8] {
        // correct PODxt lower nibble 0010 in command byte, see
        // https://github.com/torvalds/linux/blob/8508fa2e7472f673edbeedf1b1d2b7a6bb898ecc/sound/usb/line6/midibuf.c#L148
        if read_ptr[0] == 0xb2 || read_ptr[0] == 0xc2 || read_ptr[0] == 0xf2 {
            read_ptr[0] = read_ptr[0] & 0xf0;
        }

        let sysex = read_ptr[0] == 0xf0;
        if sysex {
            for i in 0 .. read_ptr.len() {
                if read_ptr[i] == 0xf7 {
                    return &read_ptr[..i + 1];
                }
            }
            return &[];

        } else {
            return read_ptr;
        }
    }
}

impl InFramer for PodXtInFramer {
    fn decode_incoming(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        // add received data to the read buffer at current read offset
        let mut read_ptr = &mut self.read_buffer[self.read_offset .. self.read_offset + bytes.len()];
        read_ptr.copy_from_slice(bytes);

        // go through the whole receive buffer from offset 0, check for
        // for messages as send them to the MIDI thread
        let process_len = self.read_offset + read_ptr.len();
        let mut process_buf = self.read_buffer[..process_len].as_mut();
        let mut process_offset = 0;
        let mut ret = vec![];
        loop {
            let process_buf = process_buf[process_offset .. process_len].as_mut();
            let buf = Self::find_message(process_buf);
            if buf.len() > 0 {
                // message found
                ret.push(buf.to_vec());
            }
            process_offset += buf.len();
            if buf.len() == 0 || process_offset == process_len { break }
        }
        if process_offset > 0 {
            // at least one message consumed
            if process_buf.len() - process_offset > 0 {
                // data left in the buffer, move it to the beginning of the read buffer
                self.read_buffer.copy_within(process_offset .. process_len, 0);
                self.read_offset = process_len - process_offset;
            } else {
                // all data consumed
                self.read_offset = 0;
            }
        } else {
            // unfinished message, adjust read offset
            self.read_offset = process_len;
        }

        ret
    }
}

/// A framer for outgoing messages using the PODxt USB MIDI protocol,
/// which essentially just sends the messages as-is
pub struct PodXtOutFramer;

impl OutFramer for PodXtOutFramer {
    fn encode_outgoing(&self, bytes: &[u8]) -> Vec<Vec<u8>> {
        vec![ bytes.to_vec() ]
    }
}

pub fn new_pod_xt_framer() -> (BoxedInFramer, BoxedOutFramer) {
    (Box::new(PodXtInFramer::new()), Box::new(PodXtOutFramer))
}