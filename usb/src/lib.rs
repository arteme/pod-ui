mod devices;
mod line6;
mod dev_handler;
mod endpoint;
mod util;
mod usb;
mod midi_framer;
mod podxt_framer;
mod framer;

use log::{error, info};
use anyhow::*;
use core::result::Result::Ok;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use once_cell::sync::Lazy;

use pod_core::midi_io::{MidiIn, MidiOut};
use regex::Regex;
use crate::dev_handler::Device;
use crate::devices::find_device;
use crate::usb::Usb;
use crate::util::usb_address_string;

enum UsbEnumeratedDevice {
    Device(Device),
    Error(String)
}

type DeviceMap = HashMap<String, Device>;

static DEVICES: Lazy<Arc<Mutex<DeviceMap>>> = Lazy::new(|| {
   Arc::new(Mutex::new(HashMap::new()))
});

static USB: OnceLock<Arc<Mutex<Usb>>> = OnceLock::new();

pub fn usb_start() -> Result<()> {
    let v = rusb::version();
    info!("libusb v{}.{}.{}.{}{}",
        v.major(), v.minor(), v.micro(), v.nano(), v.rc().unwrap_or("")
    );

    let usb = Usb::new()?;
    USB.set(Arc::new(Mutex::new(usb))).map_err(|_| anyhow!("Failed to set global USB var"))?;

    Ok(())
}

fn usb_enumerate_devices(devices: &mut DeviceMap) -> Vec<UsbEnumeratedDevice> {
    let Some(usb) = USB.get() else {
        error!("Cannot enumerate USB: usb not ready!");
        return vec![];
    };
    let usb = usb.lock().unwrap();

    info!("Enumerating USB devices...");

    let listed_devices = match usb.list_devices() {
        Ok(devices) => { devices }
        Err(err) => {
            error!("Failed to list USB devices: {err}");
            return vec![];
        }
    };

    let mut add = HashMap::new();
    let mut enumerated = Vec::with_capacity(listed_devices.len());
    let mut listed_keys = HashSet::new();
    for dev in listed_devices {
        let key = usb_address_string(dev.bus, dev.address);
        match devices.get(&key) {
            Some(dev) => {
                // device already open
                enumerated.push(UsbEnumeratedDevice::Device(dev.clone()))
            }
            None => {
                // attempt to open a new device
                let usb_dev = find_device(dev.vid, dev.pid).unwrap();
                let h = match usb.open(dev.vid, dev.pid, dev.bus, dev.address) {
                    Ok(h) => { h }
                    Err(e) => {
                        error!("Failed to open device: {}", e);
                        enumerated.push(UsbEnumeratedDevice::Error(e.to_string()));
                        continue
                    }
                };
                let dev = match Device::new(h, usb_dev) {
                    Ok(h) => { h }
                    Err(e) => {
                        error!("Filed to initialize device {:?}: {}", usb_dev.name, e);
                        enumerated.push(UsbEnumeratedDevice::Error(format!("Failed to initialize device {:?}: {}", usb_dev.name, e)));
                        continue
                    }
                };

                add.insert(key.clone(), dev.clone());
                enumerated.push(UsbEnumeratedDevice::Device(dev));
            }
        }
        listed_keys.insert(key);

    }

    let added = add.len();
    for (key, value) in add {
        devices.insert(key, value);
    }
    let keys_to_remove = devices.keys()
        .filter(|key| !listed_keys.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    let removed = keys_to_remove.len();
    for key in keys_to_remove {
        devices.remove(&key);
    }

    info!("Enumerating USB devices finished: +{added}/-{removed} entries");
    enumerated
}


pub fn usb_list_devices() -> Vec<(String, bool)> {
    let mut devices = DEVICES.lock().unwrap();
    usb_enumerate_devices(&mut devices).iter()
        .map(|i| match i {
            UsbEnumeratedDevice::Device(dev) => { (dev.name.clone(), true) }
            UsbEnumeratedDevice::Error(err) => { (err.clone(), false) }
        }).collect()
}

fn usb_remove_device(key: String) {
    let mut devices = DEVICES.lock().unwrap();
    devices.remove(&key);
}

pub fn usb_device_for_address(dev_addr: &str) -> Result<(impl MidiIn, impl MidiOut)> {
    let mut devices = DEVICES.lock().unwrap();
    let _ = usb_enumerate_devices(&mut devices);

    let port_n_re = Regex::new(r"\d+").unwrap();
    let port_id_re = Regex::new(r"\d+:\d+").unwrap();

    let found;
    if port_id_re.is_match(dev_addr) {
        found = devices.get_mut(dev_addr);
    } else if port_n_re.is_match(dev_addr) {
        let n = usize::from_str(&dev_addr)
            .with_context(|| format!("Unrecognized USB device index {:?}", dev_addr))?;
        found = devices.values_mut().nth(n);
    } else {
        bail!("Unrecognized USB device address {:?}", dev_addr);
    }

    match found {
        Some(dev) => { dev.open() }
        None => {
            bail!("USB device for address {:?} not found!", dev_addr);
        }
    }
}

pub fn usb_device_for_name(dev_name: &str) -> Result<(impl MidiIn, impl MidiOut)> {
    let mut devices = DEVICES.lock().unwrap();
    let _ = usb_enumerate_devices(&mut devices);

    let found = devices.values_mut().find(|dev| dev.name == dev_name);
    match found {
        Some(dev) => { dev.open() }
        None => {
            bail!("USB device for name {:?} not found!", dev_name);
        }
    }
}
