mod event;
mod devices;
mod line6;
mod dev_handler;
mod endpoint;

use log::{debug, error, info};
use anyhow::*;
use core::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

use rusb::{Context, Device, Hotplug, HotplugBuilder, UsbContext};
use tokio::sync::{broadcast, Notify};
use tokio::sync::broadcast::error::RecvError;
use crate::dev_handler::DevHandler;
use crate::devices::find_device;
use crate::event::*;

struct HotplugHandler {
    event_tx: broadcast::Sender<UsbEvent>,
    num_devices: Option<isize>
}

impl<T: UsbContext> Hotplug<T> for HotplugHandler {
    fn device_arrived(&mut self, device: Device<T>) {
        let Ok(desc) = device.device_descriptor() else { return };

        if find_device(desc.vendor_id(), desc.product_id()).is_some() {
            debug!("device added: {:?}", device);
            let e = DeviceAddedEvent {
                vid: desc.vendor_id(),
                pid: desc.product_id()
            };
            self.event_tx.send(UsbEvent::DeviceAdded(e)).unwrap();
        }

        if let Some(mut num) = self.num_devices.take() {
            num -= 1;
            self.num_devices = if num > 1 {
                Some(num)
            } else {
                self.event_tx.send(UsbEvent::InitDone).unwrap();
                None
            };
        }
    }

    fn device_left(&mut self, device: Device<T>) {
        let Ok(desc) = device.device_descriptor() else { return };

        if find_device(desc.vendor_id(), desc.product_id()).is_some() {
            debug!("device removed: {:?}", device);
            let e = DeviceRemovedEvent {
                vid: desc.vendor_id(),
                pid: desc.product_id()
            };
            self.event_tx.send(UsbEvent::DeviceRemoved(e)).unwrap();
        }
    }
}

static mut INIT_DONE: AtomicBool = AtomicBool::new(false);
static INIT_DONE_NOTIFY: Lazy<Arc<Notify>> = Lazy::new(|| {
    Arc::new(Notify::new())
});

pub fn usb_start() -> Result<()> {
    if !rusb::has_hotplug() {
        bail!("Libusb hotplug API not supported");
    }

    let (event_tx, mut event_rx) = broadcast::channel::<UsbEvent>(512);

    let ctx = Context::new()?;
    let hh = HotplugHandler {
        event_tx: event_tx.clone(),
        num_devices: Some(ctx.devices()?.len() as isize)
    };
    let hotplug = HotplugBuilder::new()
        .enumerate(true)
        .register(&ctx, Box::new(hh))?;

    tokio::spawn(async move {
        info!("Starting USB hotplug");
        let mut reg = Some(hotplug);
        loop {
            ctx.handle_events(None).unwrap();
            if let Some(reg) = reg.take() {
                ctx.unregister_callback(reg);
            }
        }
    });

    let ctx = Context::new()?;

    tokio::spawn(async move {
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
                UsbEvent::DeviceAdded(DeviceAddedEvent{ vid, pid }) => {
                    let usb_dev = find_device(vid, pid).unwrap();
                    //let Some(h) = rusb::open_device_with_vid_pid(vid, pid) else { continue };
                    let Some(h) = rusb::open_device_with_vid_pid(vid, pid) else { continue };
                    let mut handler = match DevHandler::new(h, usb_dev) {
                        Ok(h) => { h }
                        Err(e) => {
                            error!("Filed to initialize device {:?}: {}", usb_dev.name, e);
                            continue
                        }
                    };
                    handler.start();
                }
                UsbEvent::DeviceRemoved(_) => {}
                UsbEvent::InitDone => {
                    debug!("USB init done");
                    usb_init_set_done();
                }
            }
        }
    });

    Ok(())
}

fn usb_init_set_done()  {
    unsafe { INIT_DONE.store(true, Ordering::Relaxed) }

    INIT_DONE_NOTIFY.notify_waiters()
}

fn usb_init_done() -> bool {
    unsafe { INIT_DONE.load(Ordering::Relaxed) }
}

pub async fn usb_init_wait() -> () {
    if usb_init_done() {
        return;
    }

    INIT_DONE_NOTIFY.notified().await
}

