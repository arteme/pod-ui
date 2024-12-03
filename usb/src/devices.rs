use once_cell::sync::Lazy;
use rusb::TransferType;

pub enum UsbId {
    Device { vid: u16, pid: u16 },
    DeviceInterface { vid: u16, pid: u16, iface: u8 }
}

pub trait UsbIdOps {
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

pub struct UsbDevice {
    pub id: UsbId,
    pub name: String,
    pub alt_setting: u8,
    pub read_ep: u8,
    pub write_ep: u8,
    pub transfer_type: TransferType,
}

// based on: https://github.com/torvalds/linux/blob/8508fa2e7472f673edbeedf1b1d2b7a6bb898ecc/sound/usb/line6/pod.c
static USB_DEVICES: Lazy<Vec<UsbDevice>> = Lazy::new(|| {
    vec![
        UsbDevice {
            id: id!(0x0e41, 0x5044), name: "POD XT".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03,
            transfer_type: TransferType::Interrupt
        },
        UsbDevice {
            id: id!(0x0e41, 0x5050), name: "POD XT Pro".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03,
            transfer_type: TransferType::Interrupt
        },
        UsbDevice {
            id: id!(0x0e41, 0x4650, 0), name: "POD XT Live".into(),
            alt_setting: 1, read_ep: 0x84, write_ep: 0x03,
            transfer_type: TransferType::Interrupt
        },
        UsbDevice {
            id: id!(0x0e41, 0x4250), name: "Bass POD XT".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03,
            transfer_type: TransferType::Interrupt
        },
        UsbDevice {
            id: id!(0x0e41, 0x4252), name: "Bass POD XT Pro".into(),
            alt_setting: 5, read_ep: 0x84, write_ep: 0x03,
            transfer_type: TransferType::Interrupt
        },
        UsbDevice {
            id: id!(0x0e41, 0x4642), name: "Bass POD XT Live".into(),
            alt_setting: 1, read_ep: 0x84, write_ep: 0x03,
            transfer_type: TransferType::Interrupt
        },
        UsbDevice {
            id: id!(0x0e41, 0x5051, 1), name: "Pocket POD".into(),
            alt_setting: 0, read_ep: 0x82, write_ep: 0x02,
            transfer_type: TransferType::Bulk
        },
        UsbDevice {
            id: id!(0x0010, 0x0001), name: "POD-UI testing device".into(),
            alt_setting: 0, read_ep: 0x81, write_ep: 0x02,
            transfer_type: TransferType::Bulk
        },
    ]
});

pub fn find_device(vid: u16, pid: u16) -> Option<&'static UsbDevice> {
  USB_DEVICES.iter().find(|d| d.id.vid() == vid && d.id.pid() == pid)
}