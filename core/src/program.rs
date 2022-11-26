use crate::controller::Controller;
use crate::dump::ProgramsDump;
use crate::edit::*;
use crate::event::Origin;


pub fn store_patch_dump_ctrl_buf(edit: &EditBuffer, buffer: &mut [u8]) {
    let raw = edit.raw_locked();
    buffer.copy_from_slice(&raw);
}

pub fn load_patch_dump_ctrl<F>(edit: &mut EditBuffer, buffer: &[u8], control_value_from_buffer: F)
    where F: Fn(&mut Controller, &str, &[u8])
{
    edit.raw_locked().copy_from_slice(buffer);
    edit.load_from_raw(control_value_from_buffer);
}

pub fn store_patch_dump_ctrl(edit: &EditBuffer) -> Vec<u8> {
    let mut data = vec![0; edit.raw_locked().len()];
    store_patch_dump_ctrl_buf(edit, data.as_mut_slice());

    data
}

// --

pub fn load_patch_dump(programs_dump: &mut ProgramsDump,
                       page: usize, data: &[u8], origin: Origin) {

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

pub fn store_patch_dump(programs_dump: &ProgramsDump, page: usize) -> Vec<u8> {
    let mut data = vec![0; programs_dump.program_size()];
    store_patch_dump_buf(programs_dump, page, data.as_mut_slice());

    data
}

pub fn load_all_dump(programs_dump: &mut ProgramsDump, data: &[u8], origin: Origin) {
    let mut chunks = data.chunks(programs_dump.program_size());
    for i in 0 .. programs_dump.program_num() {
        let chunk = chunks.next().unwrap();
        load_patch_dump(programs_dump, i, chunk, origin);
    }
}

pub fn store_all_dump(programs_dump: &ProgramsDump) -> Vec<u8> {
    let mut data = vec![0; programs_dump.program_size() * programs_dump.program_num()];
    let mut chunks = data.chunks_mut(programs_dump.program_size());
    for i in 0 .. programs_dump.program_num() {
        let chunk = chunks.next().unwrap();
        store_patch_dump_buf(programs_dump, i, chunk);
    }

    data
}