// Line6-specific USB routines based on Linux source code:
// https://github.com/torvalds/linux/blob/8508fa2e7472f673edbeedf1b1d2b7a6bb898ecc/sound/usb/line6/

use std::thread::sleep;
use std::time::Duration;
use anyhow::*;
use rusb::{DeviceHandle, Direction, Recipient, request_type, RequestType, UsbContext};

pub fn line6_read_serial<T: UsbContext>(dev: &DeviceHandle<T>) -> Result<u32> {
    let mut data= [ 0u8; 4 ];
    line6_read_data(dev, 0x80d0, &mut data)?;
    Ok(u32::from_le_bytes(data))
}

const READ_WRITE_STATUS_DELAY: Duration = Duration::from_millis(2);
const READ_WRITE_MAX_RETRIES: usize = 50;

fn line6_read_data<T: UsbContext>(dev: &DeviceHandle<T>, address: u16, buf: &mut [u8]) -> Result<()> {
    let timeout = Duration::from_secs(1);

    dev.write_control(
        request_type(Direction::Out, RequestType::Vendor, Recipient::Device),
        0x67,
        (((buf.len() & 0xff) as u16) << 8) | 0x21, address,
        &[],
        timeout
    )?;

    let mut len = [0u8];
    for _i in 0..READ_WRITE_MAX_RETRIES {
        sleep(READ_WRITE_STATUS_DELAY);

        dev.read_control(
            request_type(Direction::In, RequestType::Vendor, Recipient::Device),
            0x67,
            0x0012, 0x0,
            &mut len,
            timeout
        )?;

        if len[0] != 0xff { break }
    }

    match len {
        [0xff] => {
            bail!("USB read failed after {} retries", READ_WRITE_MAX_RETRIES);
        }
        [s] if s != buf.len() as u8 => {
            bail!("USB read length mismatch: expected {} got {}", buf.len(), s);
        }
        _ => {}
    }

    dev.read_control(
        request_type(Direction::In, RequestType::Vendor, Recipient::Device),
        0x67,
        0x0013, 0x0,
        buf,
        timeout
    )?;

    Ok(())
}
