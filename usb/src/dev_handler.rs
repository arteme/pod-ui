use anyhow::*;
use core::result::Result::Ok;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use log::{error, info, trace};
use rusb::{DeviceHandle, Direction, Error, TransferType, UsbContext};
use tokio::sync::mpsc;
use crate::devices::UsbDevice;
use crate::endpoint::{configure_endpoint, Endpoint, find_endpoint};
use crate::line6::line6_read_serial;
use crate::util::usb_address_string;

pub struct Device<T: UsbContext + 'static> {
    pub name: String,
    handle: Arc<DeviceHandle<T>>,
    read_ep: Endpoint,
    write_ep: Endpoint,
    inner: Weak<DeviceInner<T>>
}

pub struct DeviceInner<T: UsbContext + 'static> {
    handle: Arc<DeviceHandle<T>>,
}

pub struct DeviceInput<T: UsbContext + 'static> {
    inner: Arc<DeviceInner<T>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>
}

pub struct DeviceOutput<T: UsbContext + 'static> {
    inner: Arc<DeviceInner<T>>
}

pub struct DevHandler<T: UsbContext> {
    handle: Arc<DeviceHandle<T>>,
    read_ep: Endpoint,
    write_ep: Endpoint,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>
}


impl <T: UsbContext + 'static> Device<T> {
    pub fn new(handle: DeviceHandle<T>, usb_dev: &UsbDevice) -> Result<Self> {
        let serial = line6_read_serial(&handle).ok()
            .map(|s| format!(" {}", s))
            .unwrap_or("".to_string());

        let address = usb_address_string(handle.device().bus_number(), handle.device().address());
        let name = format!("{}{} [{}]", &usb_dev.name, serial, address);
        info!("Found: {}", name);

        let desc = handle.device().device_descriptor()?;

        // TODO: replace with .expect?
        let Some(read_ep) = find_endpoint(&mut handle.device(), &desc, Direction::In, usb_dev.transfer_type, usb_dev.read_ep, usb_dev.alt_setting) else {
            bail!("Read end-point not found")
        };
        let Some(write_ep) = find_endpoint(&mut handle.device(), &desc, Direction::Out, usb_dev.transfer_type, usb_dev.write_ep, usb_dev.alt_setting) else {
            bail!("Write end-point not found")
        };

        Ok(Device {
            name,
            handle: Arc::new(handle),
            read_ep,
            write_ep,
            inner: Weak::new()
        })
    }

    pub fn open(&mut self) -> Result<(DeviceInput<T>, DeviceOutput<T>)> {
        if self.inner.upgrade().is_some() {
            bail!("Devide already open")
        }

        let inner = Arc::new(DeviceInner {
            handle: self.handle.clone()
        });
        self.inner = Arc::downgrade(&inner);

        let (tx, rx) = mpsc::unbounded_channel();
        let input = DeviceInput {
            inner: inner.clone(),
            rx
        };

        let output = DeviceOutput {
            inner: inner.clone()
        };

        Ok((input, output))
    }
}

impl <T: UsbContext + 'static> DeviceInner<T> {
    fn new(handle: Arc<DeviceHandle<T>>, read_ep: Endpoint, tx: mpsc::UnboundedSender<Vec<u8>>) -> Self {
        let handle_ret = handle.clone();
        let has_kernel_driver = match handle.kernel_driver_active(read_ep.iface) {
            Ok(true) => {
                handle.detach_kernel_driver(read_ep.iface).ok();
                true
            }
            _ => false
        };

        configure_endpoint(&handle, &read_ep).ok();

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let res = match read_ep.transfer_type {
                    TransferType::Bulk => {
                        handle.read_bulk(read_ep.address, &mut buf, Duration::MAX)
                    }
                    TransferType::Interrupt => {
                        handle.read_interrupt(read_ep.address, &mut buf, Duration::MAX)
                    }
                    tt => {
                        error!("Transfer type {:?} not supported!", tt);
                        break;
                    }

                };
                match res {
                    Ok(len) => {
                        let b = buf.chunks(len).next().unwrap();
                        trace!("<< {:02x?} len={}", &b, len);
                        tx.send(b.to_vec()).ok();
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

        DeviceInner {
            handle: handle_ret
        }
    }
}

impl <T: UsbContext + 'static> Drop for DeviceInner<T> {
    fn drop(&mut self) {
        // TODO: we consider that there is ever only one device, so interrupting
        //       the handle_events will only affect one DeviceInner... Can do better
        //       using own explicit context for each DeviceInner
        self.handle.context().interrupt_handle_events();
    }
}


impl <T:UsbContext + 'static> DevHandler<T> {
    pub fn new(handle: DeviceHandle<T>, usb_dev: &UsbDevice) -> Result<Self> {
        let serial = line6_read_serial(&handle).ok()
            .map(|s| format!(" {}", s))
            .unwrap_or("".to_string());

        let name = format!(
            "{}{} [usb:{}:{}]",
            &usb_dev.name, serial, handle.device().bus_number(), handle.device().address()
        );

        info!("Found: {}", name);

        let desc = handle.device().device_descriptor()?;

        // TODO: replace with .expect?
        let Some(read_ep) = find_endpoint(&mut handle.device(), &desc, Direction::In, usb_dev.transfer_type, usb_dev.read_ep, usb_dev.alt_setting) else {
            bail!("Read end-point not found")
        };
        let Some(write_ep) = find_endpoint(&mut handle.device(), &desc, Direction::Out, usb_dev.transfer_type, usb_dev.write_ep, usb_dev.alt_setting) else {
            bail!("Write end-point not found")
        };

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(DevHandler {
            handle: Arc::new(handle),
            read_ep,
            write_ep,
            tx,
            rx
        })
    }

    pub fn start(&mut self) {
        let mut handle = self.handle.clone();
        let read_ep = self.read_ep.clone();
        let tx = self.tx.clone();

        let has_kernel_driver = match handle.kernel_driver_active(read_ep.iface) {
            Ok(true) => {
                handle.detach_kernel_driver(read_ep.iface).ok();
                true
            }
            _ => false
        };

        configure_endpoint(&handle, &read_ep).ok();

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let res = match read_ep.transfer_type {
                    TransferType::Bulk => {
                        handle.read_bulk(read_ep.address, &mut buf, Duration::MAX)
                    }
                    TransferType::Interrupt => {
                        handle.read_interrupt(read_ep.address, &mut buf, Duration::MAX)
                    }
                    tt => {
                        error!("Transfer type {:?} not supported!", tt);
                        break;
                    }

                };
                match res {
                    Ok(len) => {
                        let b = buf.chunks(len).next().unwrap();
                        trace!("<< {:02x?} len={}", &b, len);
                        tx.send(b.to_vec()).ok();
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

    pub fn stop(&mut self) {
        // TODO: we consider that there is ever only one device
        self.handle.context().interrupt_handle_events();

    }

}