mod event;
mod devices;
mod line6;
mod dev_handler;
mod endpoint;
mod util;

use log::{debug, error, info, trace};
use anyhow::*;
use anyhow::Context as _;
use core::result::Result::Ok;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

use rusb::{Context, Device as UsbDevice, GlobalContext, Hotplug, HotplugBuilder, UsbContext};
use tokio::sync::{broadcast, Notify};
use tokio::sync::broadcast::error::RecvError;
use pod_core::midi_io::{MidiIn, MidiOut};
use regex::Regex;
use crate::dev_handler::Device;
use crate::devices::find_device;
use crate::event::*;
use crate::util::usb_address_string;

struct HotplugHandler {
    event_tx: broadcast::Sender<UsbEvent>,
    init_devices: Option<isize>
}

impl HotplugHandler {
    /// Notify the hotplug handler that `num` devices have been initialized.
    /// This is used for `UsbEvent::InitDone` event tracking.
    fn device_init_notify(&mut self, added: isize) {
        if let Some(mut num) = self.init_devices.take() {
            num -= added;
            self.init_devices = if num > 1 {
                Some(num)
            } else {
                self.event_tx.send(UsbEvent::InitDone).unwrap();
                None
            };
        }
    }
}

impl<T: UsbContext> Hotplug<T> for HotplugHandler {
    fn device_arrived(&mut self, device: UsbDevice<T>) {
        let Ok(desc) = device.device_descriptor() else { return };

        trace!("device arrived: {:?}", device);
        if find_device(desc.vendor_id(), desc.product_id()).is_some() {
            trace!("device added: {:?}", device);
            let e = DeviceAddedEvent {
                vid: desc.vendor_id(),
                pid: desc.product_id(),
                bus: device.bus_number(),
                address: device.address(),
            };
            self.event_tx.send(UsbEvent::DeviceAdded(e)).unwrap();
        }

        self.device_init_notify(1);
    }

    fn device_left(&mut self, device: UsbDevice<T>) {
        let Ok(desc) = device.device_descriptor() else { return };

        trace!("device left: {:?}", device);
        if find_device(desc.vendor_id(), desc.product_id()).is_some() {
            trace!("device removed: {:?}", device);
            let e = DeviceRemovedEvent {
                vid: desc.vendor_id(),
                pid: desc.product_id(),
                bus: device.bus_number(),
                address: device.address(),
            };
            self.event_tx.send(UsbEvent::DeviceRemoved(e)).unwrap();
        }
    }
}

static mut INIT_DONE: AtomicBool = AtomicBool::new(false);
static INIT_DONE_NOTIFY: Lazy<Arc<Notify>> = Lazy::new(|| {
    Arc::new(Notify::new())
});
static DEVICES: Lazy<Arc<Mutex<HashMap<String, Device<GlobalContext>>>>> = Lazy::new(|| {
   Arc::new(Mutex::new(HashMap::new()))
});

pub fn usb_start() -> Result<()> {
    let v = rusb::version();
    info!("libusb v{}.{}.{}.{}{}",
        v.major(), v.minor(), v.micro(), v.nano(), v.rc().unwrap_or("")
    );

    if !rusb::has_hotplug() {
        bail!("Libusb hotplug API not supported");
    }

    let (event_tx, mut event_rx) = broadcast::channel::<UsbEvent>(512);

    let ctx = Context::new()?;
    let num_devices = ctx.devices()?.len() as isize;
    let mut hh = HotplugHandler {
        event_tx: event_tx.clone(),
        init_devices: Some(num_devices)
    };
    hh.device_init_notify(0);
    let hotplug = HotplugBuilder::new()
        .enumerate(true)
        .register(&ctx, Box::new(hh))?;

    // libusb's handle_events may need to go on the blocking tasks queue
    tokio::task::spawn_blocking(move || {
        info!("USB hotplug thread start");
        let mut reg = Some(hotplug);
        loop {
            match ctx.handle_events(None) {
                Ok(_) => {}
                Err(e) => {
                    error!("Error in USB hotplug thread: {}", e);
                    break;
                }
            }
        }
        if let Some(reg) = reg.take() {
            ctx.unregister_callback(reg);
        }
        info!("USB hotplug thread finish");
    });

    let devices = DEVICES.clone();

    tokio::spawn(async move {
        info!("USB event RX thread start");
        loop {
            let msg = match event_rx.recv().await {
                Ok(msg) => { msg }
                Err(RecvError::Closed) => {
                    info!("Event bus closed");
                    return;
                }
                Err(RecvError::Lagged(n)) => {
                    error!("Event bus lagged: {}", n);
                    continue;
                }
            };

            match msg {
                UsbEvent::DeviceAdded(DeviceAddedEvent{ vid, pid, bus, address }) => {
                    let usb_dev = find_device(vid, pid).unwrap();
                    /*
                    let device_list = rusb::devices().unwrap();
                    let dev = device_list.iter().find(|dev| {
                        let desc = dev.device_descriptor().unwrap();
                        desc.vendor_id() == vid && desc.product_id() == pid
                    }).map(|dev| dev.open().unwrap());
                    let Some(h) = dev;
                     */

                    let Some(h) = rusb::open_device_with_vid_pid(vid, pid) else { continue };
                    let handler = match Device::new(h, usb_dev) {
                        Ok(h) => { h }
                        Err(e) => {
                            error!("Filed to initialize device {:?}: {}", usb_dev.name, e);
                            continue
                        }
                    };
                    let address = usb_address_string(bus, address);
                    usb_add_device(address, handler);
                }
                UsbEvent::DeviceRemoved(DeviceRemovedEvent{ bus, address, .. }) => {
                    let address = usb_address_string(bus, address);
                    usb_remove_device(address);

                }
                UsbEvent::InitDone => {
                    usb_init_set_done();
                }
            }
        }
        info!("USB event RX thread finish");
    });

    Ok(())
}

fn usb_init_set_done()  {
    unsafe { INIT_DONE.store(true, Ordering::Relaxed) }

    debug!("USB init done");
    INIT_DONE_NOTIFY.notify_waiters()
}

fn usb_init_done() -> bool {
    unsafe { INIT_DONE.load(Ordering::Relaxed) }
}

pub async fn usb_init_wait() {
    if usb_init_done() {
        return;
    }

    debug!("Waiting for USB init...");
    INIT_DONE_NOTIFY.notified().await;
    debug!("Waiting for USB init over");
}

pub fn usb_list_devices() -> Vec<String> {
    let devices = DEVICES.lock().unwrap();
    devices.values().map(|i| i.name.clone()).collect()
}

fn usb_add_device(key: String, device: Device<GlobalContext>) {
    let mut devices = DEVICES.lock().unwrap();
    devices.insert(key, device);
}

fn usb_remove_device(key: String) {
    let mut devices = DEVICES.lock().unwrap();
    devices.remove(&key);
}

pub fn usb_device_for_address(dev_addr: &str) -> Result<(impl MidiIn, impl MidiOut)> {
    let mut devices = DEVICES.lock().unwrap();

    let port_n_re = Regex::new(r"\d+").unwrap();
    let port_id_re = Regex::new(r"\d+:\d+").unwrap();

    let mut found = None;
    if port_id_re.is_match(dev_addr) {
        found = devices.get_mut(dev_addr);
    } else if port_n_re.is_match(dev_addr) {
        let n = usize::from_str(&dev_addr)
            .with_context(|| format!("Unrecognized USB device index {:?}", dev_addr))?;
        found = devices.values_mut().nth(n);
    } else {
        bail!("Unrecognized USB device address {:?}", dev_addr);
    }

    let Some(dev) = found.take() else {
        bail!("USB device for address {:?} not found!", dev_addr);
    };

    dev.open()
}

pub fn usb_device_for_name(dev_name: &str) -> Result<(impl MidiIn, impl MidiOut)> {
    let mut devices = DEVICES.lock().unwrap();

    let mut found = devices.values_mut().find(|dev| {
        dev.name == dev_name
    });
    let Some(dev) = found.take() else {
        bail!("USB device for name {:?} not found!", dev_name);
    };

    dev.open()
}
