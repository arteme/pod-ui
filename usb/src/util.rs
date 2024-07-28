
pub fn usb_address_string(bus: u8, address: u8) -> String {
    format!("{}:{}", bus, address)
}