use anyhow::*;
use core::result::Result::Ok;
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use async_trait::async_trait;
use log::{debug, error, info, trace};
use rusb::Direction;
use tokio::sync::mpsc;
use pod_core::midi_io::{MidiIn, MidiOut};
use crate::devices::UsbDevice;
use crate::endpoint::{Endpoint, find_endpoint};
use crate::framer::{BoxedInFramer, BoxedOutFramer};
use crate::line6::line6_read_serial;
use crate::midi_framer::new_usb_midi_framer;
use crate::podxt_framer::new_pod_xt_framer;
use crate::usb::{DeviceHandle, SubmittedTransfer, Transfer, TransferCommand};
use crate::util::usb_address_string;

#[derive(Clone)]
pub struct Device {
    pub name: String,
    handle: Arc<DeviceHandle>,
    read_ep: Endpoint,
    write_ep: Endpoint,
    inner: Weak<DeviceInner>
}

struct DevOpenState {
    /// Active configuration when opening the device
    config: u8,
    /// Kernel driver attach status per interface
    attach: Vec<(u8, bool)>
}

pub struct DeviceInner {
    name: String,
    handle: Arc<DeviceHandle>,
    write_ep: Endpoint,
    closed: Arc<AtomicBool>,
    read: SubmittedTransfer,
    kernel_state: DevOpenState,
    out_framer: BoxedOutFramer
}

pub struct DeviceInput {
    inner: Arc<DeviceInner>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>
}

pub struct DeviceOutput {
    inner: Arc<DeviceInner>
}

const READ_DURATION: Duration = Duration::from_millis(10 * 1000);
const WRITE_DURATION: Duration = Duration::from_millis(10 * 1000);

impl Device {
    pub fn new(handle: DeviceHandle, usb_dev: &UsbDevice) -> Result<Self> {
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

    pub fn open(&mut self) -> Result<(DeviceInput, DeviceOutput)> {
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
        )?);
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

impl DeviceInner {
    fn new(name: String, handle: Arc<DeviceHandle>,
           read_ep: Endpoint, write_ep: Endpoint,
           tx: mpsc::UnboundedSender<Vec<u8>>) -> Result<Self> {

        let kernel_state = Self::detach_kernel_driver(&handle).map_err(|e| {
            anyhow!("Failed to detach kernel driver when opening device: {e}")
        })?;

        handle.reset().map_err(|e| {
            error!("Failed to reset USB device: {}", e);
        }).ok();

        handle.set_active_configuration(read_ep.config).map_err(|e| {
            error!("Set active config error: {}", e);
        }).ok();

        Self::claim_interfaces(&handle, &kernel_state);

        if read_ep.setting != 0 {
            handle.set_alternate_setting(read_ep.iface, read_ep.setting).map_err(|e| {
                error!("Set alt setting error: {}", e);
            }).ok();
        }

        let closed = Arc::new(AtomicBool::new(false));
        let (mut framer, out_framer) = Self::init_framers(&handle, &read_ep);

        let mut read_transfer = Transfer::new_bulk(&handle, read_ep.address, 1024);
        read_transfer.set_timeout(READ_DURATION);
        read_transfer.set_callback(move |buf| {
            let Some(buf) = buf else {
                // read transfer cancelled, nothing to do here
                trace!("<< failed or cancelled");
                return TransferCommand::Drop // doesn't really matter what we return
            };

            if buf.len() == 0 {
                // read timed out, continue
                return TransferCommand::Resubmit
            }
            trace!("<< {:02x?} len={}", &buf, buf.len());

            framer.decode_incoming(buf).into_iter().for_each(|msg| {
                // message found
                trace!("<< msg {:02x?} len={}", &msg, msg.len());
                match tx.send(msg) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("USB read thread tx failed: {}", e);
                    }
                };
            });
            TransferCommand::Resubmit
        });
        let read = read_transfer.submit()
            .map_err(|e| {
                Self::close_inner(&handle, &kernel_state);
                anyhow!("Failed to set up read thread: {e}")
            })?;

        Ok(DeviceInner {
            name,
            handle,
            closed,
            write_ep,
            read,
            kernel_state,
            out_framer
        })
    }

    fn send(&self, bytes: &[u8]) -> Result<()> {
        if self.closed.load(Ordering::Relaxed) {
            bail!("Device already closed");
        }

        trace!(">> msg {:02x?} len={}", &bytes, bytes.len());

        self.out_framer.encode_outgoing(bytes).into_iter().map(|msg| {
            let mut transfer = Transfer::new_bulk_with_data(&self.handle, self.write_ep.address, &msg);
            transfer.set_timeout(WRITE_DURATION);
            transfer.set_callback(|buf| {
                if let Some(buf) = buf {
                    trace!(">> {:02x?} len={}", buf, buf.len());
                } else {
                    trace!(">> failed or cancelled");
                }
                TransferCommand::Drop
            });
            transfer.submit()
                .map(|_| ())
                .map_err(|e| anyhow!("USB write transfer failed: {}", e))
        }).collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.read.cancel().ok();

        Self::close_inner(&self.handle, &self.kernel_state);
    }

    fn close_inner(handle: &DeviceHandle, state: &DevOpenState) {
        Self::release_interfaces(handle, state);
        Self::attach_kernel_driver(handle, state);
    }

    fn detach_kernel_driver(handle: &DeviceHandle) -> Result<DevOpenState> {
        let dev = handle.device();
        let config = handle.active_configuration()?;
        let desc = dev.active_config_descriptor()?;
        let attach = desc.interfaces()
            .map(|iface| {
                let num = iface.number();
                let kernel_driver_attached = handle.kernel_driver_active(num)
                    .ok().unwrap_or(false);

                debug!("Kernel driver detach (iface={}): attached={}", num, kernel_driver_attached);
                if kernel_driver_attached {
                    handle.detach_kernel_driver(num).map_err(|e| {
                        error!("Failed to detach kernel driver (iface={}): {}", num, e);
                    }).ok();
                }

                (num, kernel_driver_attached)
            })
            .collect::<Vec<_>>();
        Ok(DevOpenState { config, attach })
    }

    fn attach_kernel_driver(handle: &DeviceHandle, state: &DevOpenState) {
        for (num, kernel_driver_attached) in state.attach.iter() {
            debug!("Kernel driver attach (iface={}): attached={}", num, kernel_driver_attached);
            if *kernel_driver_attached {
                handle.attach_kernel_driver(*num).map_err(|e| {
                    error!("Failed to attach kernel driver (iface={}): {}", num, e);
                }).ok();
            }
        }
    }

    fn claim_interfaces(handle: &DeviceHandle, state: &DevOpenState) {
        for (num, _) in state.attach.iter() {
            debug!("Claiming interface (iface={})", num);
            handle.claim_interface(*num).map_err(|e| {
                error!("Failed to claim interface (iface{}): {}", num, e);
            }).ok();
        }
    }

    fn release_interfaces(handle: &DeviceHandle, state: &DevOpenState) {
        for (num, _) in state.attach.iter() {
            debug!("Releasing interface (iface={})", num);
            handle.release_interface(*num).map_err(|e| {
                error!("Failed to release interface (iface{}): {}", num, e);
            }).ok();
        }
    }

    fn init_framers(handle: &DeviceHandle, endpoint: &Endpoint) -> (BoxedInFramer, BoxedOutFramer) {
        let dev = handle.device();
        let desc = dev.active_config_descriptor().ok();

        let mut class_code = 0;
        let mut sub_class_code = 0;
        if let Some(desc) = desc {
            for id in desc.interfaces().flat_map(|i| i.descriptors()) {
                if id.interface_number() == endpoint.iface && id.setting_number() == endpoint.setting {
                    class_code = id.class_code();
                    sub_class_code = id.sub_class_code();
                }

            }
        }

        match (class_code, sub_class_code) {
            // class = 1 (audio), sub-class = 3 (MIDI streaming)
            (1, 3) => {
                info!("Using USB-MIDI framer");
                new_usb_midi_framer()
            }
            _ => {
                info!("Using PodXT USB framer");
                new_pod_xt_framer()
            }
        }
    }
}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        self.close();
    }
}

#[async_trait]
impl MidiIn for DeviceInput {
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await
    }

    fn close(&mut self) {
        debug!("midi in close - nop");
    }
}

#[async_trait]
impl MidiOut for DeviceOutput {
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.send(bytes)
    }

    fn close(&mut self) {
        debug!("midi out close - nop");
    }
}