use crate::controller::Controller;
use crate::model::AbstractControl;
use crate::store::Store;

use log::*;

pub fn load_dump(controller: &mut Controller, data: &[u8], origin: u8) {
    for (name, control) in controller.config.controls.clone().iter() {
        let addr = control.get_addr();
        if addr.is_none() { continue };

        let (addr, bits) = addr.unwrap();
        let v = data[addr as usize];
        controller.set(name, v as u16, origin);
    }
}

pub fn dump(controller: &Controller) -> Vec<u8> {
    let mut data = vec![0; controller.config.program_size];
    for (name, control) in controller.config.controls.clone().iter() {
        let addr = control.get_addr();
        if addr.is_none() { continue };

        let (addr, bits) = addr.unwrap();
        let v = controller.get(name);
        v.map(|v| data[addr as usize] = v as u8)
            .or_else(|| { warn!("Control '{}' has None value!", name); None });
    }

    data
}
