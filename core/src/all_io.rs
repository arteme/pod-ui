#[cfg(feature = "usb")]
use crate::midi_io::*;
use crate::usb_io::*;

#[cfg(feature = "usb")]
fn usb_ports() -> anyhow::Result<Vec<String>> {
    Usb::ports()
}

#[cfg(not(feature = "usb"))]
fn usb_ports() -> anyhow::Result<Vec<String>> {
    Ok(vec![])
}

pub fn midi_in_ports() -> anyhow::Result<Vec<String>> {
    let mut midi_ports = MidiIn::ports()?;
    let usb_ports = Usb::ports()?;

    midi_ports.extend(usb_ports);
    Ok(midi_ports)

}

pub fn midi_out_ports() -> anyhow::Result<Vec<String>> {
    let mut midi_ports = MidiOut::ports()?;
    let usb_ports = Usb::ports()?;

    midi_ports.extend(usb_ports);
    Ok(midi_ports)
}
