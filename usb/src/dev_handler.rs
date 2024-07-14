use anyhow::*;
use core::result::Result::Ok;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::{error, info, trace};
use rusb::{DeviceHandle, Direction, Error, UsbContext};
use crate::devices::UsbDevice;
use crate::endpoint::{configure_endpoint, Endpoint, find_endpoint};
use crate::line6::line6_read_serial;

pub struct DevHandler<T: UsbContext> {
    handle: Arc<Mutex<DeviceHandle<T>>>,
    read_ep: Endpoint,
    write_ep: Endpoint
}

impl <T:UsbContext + 'static> DevHandler<T> {
    pub fn new(handle: DeviceHandle<T>, usb_dev: &UsbDevice) -> Result<Self> {
        let serial = line6_read_serial(&handle).ok()
            .map(|s| format!(" {}", s))
            .unwrap_or("".to_string());

        let name = format!(
            "{}{} [usb:{:#04x}:{:#04x}]",
            &usb_dev.name, serial, handle.device().bus_number(), handle.device().address()
        );

        info!("Found: {}", name);

        let desc = handle.device().device_descriptor()?;

        let Some(read_ep) = find_endpoint(&mut handle.device(), &desc, Direction::In, usb_dev.read_ep, usb_dev.alt_setting) else {
            bail!("Read end-point not found")
        };
        let Some(write_ep) = find_endpoint(&mut handle.device(), &desc, Direction::Out, usb_dev.write_ep, usb_dev.alt_setting) else {
            bail!("Write end-point not found")
        };

        Ok(DevHandler {
            handle: Arc::new(Mutex::new(handle)),
            read_ep,
            write_ep
        })
    }

    pub fn start(&mut self) {
        let handle = self.handle.clone();
        let read_ep = self.read_ep.clone();

        tokio::spawn(async move {
            let mut handle = handle.lock().unwrap();

            let has_kernel_driver = match handle.kernel_driver_active(read_ep.iface) {
                Ok(true) => {
                    handle.detach_kernel_driver(read_ep.iface).ok();
                    true
                }
                _ => false
            };

            configure_endpoint(&mut handle, &read_ep).ok();
            let mut buf = [0u8; 1024];
            loop {
                match handle.read_interrupt(read_ep.address, &mut buf, Duration::MAX) {
                    Ok(len) => {
                        let b = buf.chunks(len).next().unwrap();
                        trace!("<< {:02x?} len={}", &b, len);
                    }
                    Err(e) => {
                        error!("USB read failed: {}", e);
                        match e {
                            Error::Busy | Error::Timeout | Error::Overflow => { continue }
                            _ => { break }
                        }
                    }
                }
            }
        });
    }

}