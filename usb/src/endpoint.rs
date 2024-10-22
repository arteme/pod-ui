// from rusb example code

use core::result::Result::Ok;
use rusb::{Device, DeviceDescriptor, Direction, TransferType, UsbContext};

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub config: u8,
    pub iface: u8,
    pub setting: u8,
    pub address: u8,
    pub transfer_type: TransferType
}

pub fn find_endpoint<T: UsbContext>(
    device: &Device<T>,
    device_desc: &DeviceDescriptor,
    direction: Direction,
    transfer_type: TransferType,
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
                        endpoint_desc.transfer_type() == transfer_type &&
                        endpoint_desc.address() == address
                    {
                        return Some(Endpoint {
                            config: config_desc.number(),
                            iface: interface_desc.interface_number(),
                            setting: interface_desc.setting_number(),
                            address: endpoint_desc.address(),
                            transfer_type: endpoint_desc.transfer_type(),
                        });
                    }
                }
            }
        }
    }

    None
}