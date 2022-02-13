use std::borrow::BorrowMut;
use futures_util::StreamExt;
use crate::model::{AbstractControl, Config, Control};
use crate::store::Store;

use log::*;
use crate::controller::Controller;
use crate::dump::ProgramsDump;
use crate::names::ProgramNames;
use crate::raw::Raw;

fn ordered_controls(controller: &Controller) -> Vec<(String, Control)> {
    let mut refs = controller.controls.iter()
        .filter(|(_,c)| c.get_addr().is_some())
        .map(|(n,c)| (n.clone(),c.clone())).collect::<Vec<_>>();
    refs.sort_by(|a,b| {
        Ord::cmp(&b.1.get_addr().unwrap().0, &a.1.get_addr().unwrap().0)
    });
    refs
}

fn store_patch_dump_ctrl_buf(controller: &Controller, buffer: &mut [u8]) {
    for (name, control) in ordered_controls(controller) {
        let value = controller.get(&name).unwrap();
        let (addr, len) = control.get_addr().unwrap();
        let addr = addr as usize;
        match len {
            1 => {
                if value > u8::MAX as u16 {
                    warn!("Control {:?} value {} out of bounds!", name, value);
                }
                buffer[addr] = value as u8;
            }
            2 => {
                buffer[addr] = ((value >> 8) & 0xff) as u8;
                buffer[addr + 1] = (value & 0xff) as u8;
            }
            n => {
                error!("Control width {} not supported!", n)
            }
        }
    }
}

pub fn load_patch_dump_ctrl(controller: &mut Controller, buffer: &[u8], origin: u8) {
    for (name, control) in ordered_controls(controller) {
        let (addr, len) = control.get_addr().unwrap();
        let addr = addr as usize;
        let value = match len {
            1 => {
                buffer[addr] as u16
            }
            2 => {
                let a = buffer[addr] as u16;
                let b = buffer[addr + 1] as u16;
                (a << 8) | b
            }
            n => {
                error!("Control width {} not supported!", n);
                0u16
            }
        };
        controller.set(&name, value, origin);
    }
}

pub fn store_patch_dump_ctrl(controller: &Controller, config: &Config) -> Vec<u8> {
    let mut data = vec![0; config.program_size];
    store_patch_dump_ctrl_buf(controller, data.as_mut_slice());

    data
}

// --

pub fn load_patch_dump(programs_dump: &mut ProgramsDump,
                       page: usize, data: &[u8], origin: u8) {

    let program_buffer = programs_dump.data_mut(page);
    if program_buffer.is_none() {
        return;
    }

    let program_buffer = program_buffer.unwrap();
    program_buffer.copy_from_slice(data);
    programs_dump.update_name_from_data(page, origin);
}

pub fn store_patch_dump_buf(programs_dump: &ProgramsDump, page: usize, buffer: &mut [u8]) {
    let program_buffer = programs_dump.data(page);
    if program_buffer.is_none() {
        return;
    }

    let program_buffer = program_buffer.unwrap();
    buffer.copy_from_slice(program_buffer);
}

pub fn store_patch_dump(programs_dump: &ProgramsDump, page: usize, config: &Config) -> Vec<u8> {
    let mut data = vec![0; config.program_size];
    store_patch_dump_buf(programs_dump, page, data.as_mut_slice());

    data
}

pub fn load_all_dump(programs_dump: &mut ProgramsDump,
                     data: &[u8], config: &Config, origin: u8) {
    let mut chunks = data.chunks(config.program_size);
    for i in 0 .. config.program_num {
        let chunk = chunks.next().unwrap();
        load_patch_dump(programs_dump, i, chunk, origin);
    }
}

pub fn store_all_dump(programs_dump: &ProgramsDump, config: &Config) -> Vec<u8> {
    let mut data = vec![0; config.program_size * config.program_num];
    let mut chunks = data.chunks_mut(config.program_size);
    for i in 0 .. config.program_num {
        let chunk = chunks.next().unwrap();
        store_patch_dump_buf(programs_dump, i, chunk);
    }

    data
}