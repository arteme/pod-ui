use anyhow::*;
use core::result::Result::Ok;
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use async_trait::async_trait;
use log::{debug, error, info, trace};
use rusb::{DeviceHandle, Direction, Error, TransferType, UsbContext};
use tokio::sync::mpsc;
use pod_core::midi_io::{MidiIn, MidiOut};
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
    name: String,
    handle: Arc<DeviceHandle<T>>,
    write_ep: Endpoint,
    closed: Arc<AtomicBool>,
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

const READ_DURATION: Duration = Duration::from_millis(500);

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
            bail!("Device already open")
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let inner = Arc::new(DeviceInner::new(
            self.name.clone(),
            self.handle.clone(),
            self.read_ep.clone(),
            self.write_ep.clone(),
            tx
        ));
        self.inner = Arc::downgrade(&inner);

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
    fn new(name: String, handle: Arc<DeviceHandle<T>>,
           read_ep: Endpoint, write_ep: Endpoint,
           tx: mpsc::UnboundedSender<Vec<u8>>) -> Self {
        let has_kernel_driver = match handle.kernel_driver_active(read_ep.iface) {
            Ok(true) => {
                handle.detach_kernel_driver(read_ep.iface).ok();
                true
            }
            _ => false
        };

        configure_endpoint(&handle, &read_ep).ok();

        let closed = Arc::new(AtomicBool::new(false));

        // libusb's reads DEFINITELY need to go on the blocking tasks queue
        tokio::task::spawn_blocking({
            let name = name.clone();
            let handle = handle.clone();
            let closed = closed.clone();

            move || {
                debug!("USB read thread {:?} start", name);

                let mut buf = [0u8; 1024];
                while !closed.load(Ordering::Relaxed) {
                    let res = match read_ep.transfer_type {
                        TransferType::Bulk => {
                            handle.read_bulk(read_ep.address, &mut buf, READ_DURATION)
                        }
                        TransferType::Interrupt => {
                            handle.read_interrupt(read_ep.address, &mut buf, READ_DURATION)
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
                            match tx.send(b.to_vec()) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("USB read thread tx failed: {}", e);
                                }
                            };
                        }
                        Err(e) => {
                            match e {
                                Error::Busy | Error::Timeout | Error::Overflow => { continue }
                                _ => {
                                    error!("USB read failed: {}", e);
                                    break
                                }
                            }
                        }
                    }
                }

                debug!("USB read thread {:?} finish", name);
            }
        });

        DeviceInner {
            name,
            handle,
            closed,
            write_ep
        }
    }

    fn send(&self, bytes: &[u8]) -> Result<()> {
        trace!(">> {:02x?} len={}", bytes, bytes.len());
        let res = match self.write_ep.transfer_type {
            TransferType::Bulk => {
                self.handle.write_bulk(self.write_ep.address, bytes, Duration::MAX)
            }
            /*
            TransferType::Interrupt => {
                self.handle.write_bulk(self.write_ep.address, buf, Duration::MAX)
            }*/
            tt => {
                bail!("Transfer type {:?} not supported!", tt);
            }
        };

        res.map(|_| ()).map_err(|e| anyhow!("USB write failed: {}", e))
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

impl <T: UsbContext + 'static> Drop for DeviceInner<T> {
    fn drop(&mut self) {
        debug!("DeviceInner for {:?} dropped", &self.name);
        self.close();
    }
}

#[async_trait]
impl <T: UsbContext> MidiIn for DeviceInput<T> {
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await
    }

    fn close(&mut self) {
        debug!("midi in close");
    }
}

#[async_trait]
impl <T: UsbContext> MidiOut for DeviceOutput<T> {
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.send(bytes)
    }

    fn close(&mut self) {
        debug!("midi out close");
    }
}