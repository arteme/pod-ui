use std::thread::sleep;
use std::time::Duration;
use anyhow::*;
use core::result::Result::Ok;
use once_cell::sync::Lazy;
use rusb::{Device, DeviceHandle, DeviceList, Direction, Recipient, request_type, RequestType, UsbContext};
use crate::midi_io::MidiPorts;

pub struct Usb;

enum UsbId {
    Device { vid: u16, pid: u16 },
    DeviceInterface { vid: u16, pid: u16, iface: u8 }
}

trait UsbIdOps {
    fn vid(&self) -> u16;
    fn pid(&self) -> u16;
}

impl UsbIdOps for UsbId {
    fn vid(&self) -> u16 {
        match self {
            UsbId::Device { vid, .. } => *vid,
            UsbId::DeviceInterface { vid, .. } => *vid
        }
    }

    fn pid(&self) -> u16 {
        match self {
            UsbId::Device { pid, .. } => *pid,
            UsbId::DeviceInterface { pid, .. } => *pid
        }
    }
}

macro_rules! id {
    ($vid:tt, $pid:tt) => (
        UsbId::Device { vid: $vid, pid: $pid }
    );
    ($vid:tt, $pid:tt, $iface:tt) => (
        UsbId::DeviceInterface { vid: $vid, pid: $pid, iface: $iface }
    );
}

struct UsbDevice {
    id: UsbId,
    name: String,
    alt_setting: u8,
    read_ep: u8,
    write_ep: u8
}

// based on: https://github.com/torvalds/linux/blob/8508fa2e7472f673edbeedf1b1d2b7a6bb898ecc/sound/usb/line6/pod.c
static USB_DEVICES: Lazy<Vec<UsbDevice>> = Lazy::new(|| {
    vec![
        UsbDevice {
            id: id!(0x0e41, 0x5044), name: "POD XT".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03
        },
        UsbDevice {
            id: id!(0x0e41, 0x5050), name: "POD XT Pro".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03
        },
        UsbDevice {
            id: id!(0x0e41, 0x4650, 0), name: "POD XT Live".into(),
            alt_setting: 1, read_ep: 0x84, write_ep: 0x03
        },
        UsbDevice {
            id: id!(0x0e41, 0x4250), name: "Bass POD XT".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03
        },
        UsbDevice {
            id: id!(0x0e41, 0x4252), name: "Bass POD XT Pro".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03
        },
        UsbDevice {
            id: id!(0x0e41, 0x4642), name: "Bass POD XT Live".into(),
            alt_setting: 1, read_ep: 0x84, write_ep: 0x03
        },
        UsbDevice {
            id: id!(0x0e41, 0x5051, 1), name: "Pocket POD".into(),
            alt_setting: 1, read_ep: 0x82, write_ep: 0x02
        },
    ]
});



impl MidiPorts for Usb {
    fn all_ports() -> anyhow::Result<Vec<String>> {
        let timeout = Duration::from_secs(1);
        let mut ports = vec![];

        for dev in DeviceList::new()?.iter() {
            let Ok(desc) = dev.device_descriptor() else { continue };
            let Some(d) = USB_DEVICES.iter().find(|d|
                desc.vendor_id() == d.id.vid() && desc.product_id() == d.id.pid()
            ) else { continue };
            let Ok(handle) = dev.open() else { continue };

            let serial = line6_read_serial(&handle).ok()
                .map(|s| format!(" {}", s))
                .unwrap_or("".to_string());

            let name = format!(
                "{}{} [usb:{:#04x}:{:#04x}]",
                &d.name, serial, dev.bus_number(), dev.address()
            );

            ports.push(name);

            println!("num configs: {}", desc.num_configurations());

            let Ok(config_desc) = dev.config_descriptor(d.alt_setting) else { continue };
            println!("{:?}", config_desc);
        }

        Ok(ports)
    }

    fn ports() -> Result<Vec<String>> {
        <Usb as MidiPorts>::all_ports()
    }
}

fn line6_read_serial<T: UsbContext>(dev: &DeviceHandle<T>) -> Result<u32> {
    let mut data: [u8; 4] = [ 0u8, 0u8, 0u8, 0u8 ];
    line6_read_data(dev, 0x80d0, &mut data)?;
    Ok(u32::from_le_bytes(data))
}

const READ_WRITE_STATUS_DELAY: Duration = Duration::from_millis(2);
const READ_WRITE_MAX_RETRIES: usize = 50;

fn line6_read_data<T: UsbContext>(dev: &DeviceHandle<T>, address: u16, buf: &mut [u8]) -> Result<()> {
    let timeout = Duration::from_secs(1);

    dev.write_control(
        request_type(Direction::Out, RequestType::Vendor, Recipient::Device),
        0x67,
        (((buf.len() & 0xff) as u16) << 8) | 0x21, address,
        &[],
        timeout
    )?;

    let mut len = [0u8];
    for i in 0..READ_WRITE_MAX_RETRIES {
        sleep(READ_WRITE_STATUS_DELAY);

        dev.read_control(
            request_type(Direction::In, RequestType::Vendor, Recipient::Device),
            0x67,
            0x0012, 0x0,
            &mut len,
            timeout
        )?;

        if len[0] != 0xff { break }
    }

    match len {
        [0xff] => {
            bail!("USB read failed after {} retries", READ_WRITE_MAX_RETRIES);
        }
        [s] if s != buf.len() as u8 => {
            bail!("USB read length mismatch: expected {} got {}", buf.len(), s);
        }
        _ => {}
    }

    dev.read_control(
        request_type(Direction::In, RequestType::Vendor, Recipient::Device),
        0x67,
        0x0013, 0x0,
        buf,
        timeout
    )?;

    Ok(())
}