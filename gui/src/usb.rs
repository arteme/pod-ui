
#[cfg(feature = "usb")]
pub use imp::*;

#[cfg(not(feature = "usb"))]
pub use nop::*;

#[cfg(feature = "usb")]
mod imp {
    use anyhow::*;
    use core::result::Result::Ok;
    use log::warn;
    use pod_core::midi::Channel;
    use pod_core::midi_io;
    use pod_core::midi_io::{AutodetectResult, MidiIn, MidiOut};

    pub fn start_usb() {
        pod_usb::usb_start().unwrap();
    }

    pub fn usb_list_devices() -> Vec<(String, bool)> {
        pod_usb::usb_list_devices()
    }

    pub fn usb_open_addr(addr: &str) -> Result<(impl MidiIn, impl MidiOut)> {
        pod_usb::usb_device_for_address(addr)
    }

    pub fn usb_open_name(name: &str) -> Result<(impl MidiIn, impl MidiOut)> {
        pod_usb::usb_device_for_name(name)
    }

    /**
     * USB-specific auto-detect that is called whenever MIDI autodetect
     * didn't return any results.
    */
    pub async fn autodetect() -> Result<AutodetectResult> {
        let devices = usb_list_devices();
        if devices.is_empty() {
            bail!("No compatible USB devices found")
        }

        for (name, is_ok) in usb_list_devices() {
            if !is_ok { continue }

            let (in_port, out_port) = match usb_open_name(&name) {
                Ok(r) => r,
                Err(e) => {
                    warn!("USB auto-detect failed for device {name:?}: {e}");
                    continue;
                }
            };
            let res = midi_io::autodetect_with_ports(
                vec![Box::new(in_port)], vec![Box::new(out_port)], Some(Channel::num(0))
            ).await;
            if res.is_ok() {
                return res;
            }
        }

        bail!("USB auto-detect failed");
    }

    pub const fn autodetect_supported() -> bool {
        true
    }
}

mod nop {
    use anyhow::*;
    use pod_core::midi_io::{AutodetectResult, MidiInPort, MidiOutPort};

    fn start_usb() {
    }

    fn usb_list_devices() -> Vec<(String, bool)> {
        vec![]
    }

    fn usb_open_addr(_addr: &str) -> Result<(MidiInPort, MidiOutPort)> {
        unimplemented!()
    }

    fn usb_open_name(_addr: &str) -> Result<(MidiInPort, MidiOutPort)> {
        unimplemented!()
    }

    pub async fn autodetect() -> Result<AutodetectResult> {
        unimplemented!()
    }

    pub const fn autodetect_supported() -> bool {
        false
    }
}
