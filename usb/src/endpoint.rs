// from rusb example code

use anyhow::*;
use core::result::Result::Ok;
use rusb::{Device, DeviceDescriptor, DeviceHandle, Direction, TransferType, UsbContext};

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub config: u8,
    pub iface: u8,
    pub setting: u8,
    pub address: u8,
}

pub fn configure_endpoint<T: UsbContext>(
    handle: &mut DeviceHandle<T>,
    endpoint: &Endpoint,
) -> Result<()> {
    handle.set_active_configuration(endpoint.config)?;
    handle.claim_interface(endpoint.iface)?;
    handle.set_alternate_setting(endpoint.iface, endpoint.setting)?;
    Ok(())
}



pub fn find_endpoint<T: UsbContext>(
    device: &mut Device<T>,
    device_desc: &DeviceDescriptor,
    direction: Direction,
    address: u8,
    setting: u8,
) -> Option<Endpoint> {
    for n in 0..device_desc.num_configurations() {
        let Ok(config_desc) = device.config_descriptor(n) else { continue };

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                if interface_desc.setting_number() != setting { continue }

                for endpoint_desc in interface_desc.endpoint_descriptors() {
                    if endpoint_desc.direction() == direction &&
                        endpoint_desc.transfer_type() == TransferType::Interrupt &&
                        endpoint_desc.address() == address
                    {
                        return Some(Endpoint {
                            config: config_desc.number(),
                            iface: interface_desc.interface_number(),
                            setting: interface_desc.setting_number(),
                            address: endpoint_desc.address(),
                        });
                    }
                }
            }
        }
    }

    None
}