
#[cfg(feature = "usb")]
pub use imp::*;

#[cfg(not(feature = "usb"))]
pub use nop::*;

#[cfg(feature = "usb")]
mod imp {
    use anyhow::*;
    use futures::executor;
    use pod_core::midi_io;
    use pod_core::midi_io::{box_midi_in, box_midi_out, BoxedMidiIn, BoxedMidiOut, MidiIn, MidiOut};
    use pod_core::model::Config;

    pub fn start_usb() {
        pod_usb::usb_start().unwrap();
        executor::block_on(
            pod_usb::usb_init_wait()
        );
    }

    pub fn usb_list_devices() -> Vec<String> {
        pod_usb::usb_list_devices()
    }

    pub fn usb_open_addr(addr: &str) -> Result<(impl MidiIn, impl MidiOut)> {
        pod_usb::usb_device_for_address(addr)
    }

    pub fn usb_open_name(name: &str) -> Result<(impl MidiIn, impl MidiOut)> {
        pod_usb::usb_device_for_name(name)
    }
}

mod nop {
    use anyhow::*;
    use pod_core::midi_io::{BoxedMidiIn, BoxedMidiOut, MidiInPort, MidiOutPort};
    use pod_core::model::Config;

    fn start_usb() {
    }

    fn usb_list_devices() -> Vec<String> {
        vec![]
    }

    fn usb_open_addr(_addr: &str) -> Result<(MidiInPort, MidiOutPort)> {
        unimplemented!()
    }

    fn usb_open_name(_addr: &str) -> Result<(MidiInPort, MidiOutPort)> {
        unimplemented!()
    }
}
