use log::*;
use pod_core::context::Ctx;
use pod_core::controller::*;
use pod_core::event::*;
use pod_core::event::Origin::MIDI;
use pod_core::generic;
use pod_core::handler::Handler;
use pod_core::midi::MidiMessage;
use pod_core::model::AbstractControl;

pub struct Pod2Handler;

fn midi_in_buffer_handler(ctx: &Ctx, buffer: Buffer, n_programs: usize, ver: &u8, data: &Vec<u8>) {
    if *ver != 0 {
        error!("Program dump version not supported: {}", ver);
    }
    if data.len() != ctx.config.program_size * n_programs {
        error!("Program size mismatch: expected {}, got {}",
                       ctx.config.program_size, data.len());
        return;
    }
    let e = BufferDataEvent {
        buffer,
        origin: MIDI,
        request: MIDI,
        data: data.clone()
    };
    ctx.app_event_tx.send_or_warn(AppEvent::BufferData(e));
}

impl Handler for Pod2Handler {
    fn midi_in_handler(&self, ctx: &Ctx, midi_message: &MidiMessage) {
        generic::midi_in_handler(ctx, midi_message);

        match midi_message {
            MidiMessage::ProgramEditBufferDumpRequest => {
                let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: MIDI };
                ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
            }
            MidiMessage::ProgramEditBufferDump { ver, data } => {
                midi_in_buffer_handler(ctx, Buffer::EditBuffer, 1, ver, data);
            }
            MidiMessage::ProgramPatchDumpRequest { patch } => {
                let e = BufferLoadEvent { buffer: Buffer::Program(*patch as usize), origin: MIDI };
                ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
            }
            MidiMessage::ProgramPatchDump { patch, ver, data } => {
                midi_in_buffer_handler(ctx, Buffer::Program(*patch as usize),
                                       1, ver, data);
            }
            MidiMessage::AllProgramsDumpRequest => {
                let e = BufferLoadEvent { buffer: Buffer::All, origin: MIDI };
                ctx.app_event_tx.send_or_warn(AppEvent::Load(e));
            }
            MidiMessage::AllProgramsDump { ver, data } => {
                midi_in_buffer_handler(ctx, Buffer::All,
                                       ctx.config.program_num, ver, data);
            }
            _ => {}
        }
    }

    fn control_value_from_buffer(&self, controller: &mut Controller, name: &str, buffer: &[u8]) {
        let Some(control) = controller.get_config(name) else {
            return;
        };
        let Some((addr, len)) = control.get_addr() else {
            return; // skip virtual controls
        };
        let addr = addr as usize;
        let value = match len {
            1 => {
                buffer[addr] as u32
            }
            2 => {
                let a = buffer[addr] as u32;
                let b = buffer[addr + 1] as u32;
                (a << 8) | b
            }
            4 => {
                let a = buffer[addr] as u32;
                let b = buffer[addr + 1] as u32;
                let c = buffer[addr + 2] as u32;
                let d = buffer[addr + 3] as u32;
                (a << 24) | (b << 16) | (c << 8)  | d
            }
            n => {
                error!("Control width {} not supported!", n);
                0u32
            }
        };

        let value = control.value_from_buffer(value);
        controller.set(&name, value, StoreOrigin::NONE);
    }

    fn control_value_to_buffer(&self, controller: &Controller, name: &str, buffer: &mut [u8]) {
        let Some(control) = controller.get_config(name) else {
            return;
        };
        let Some((addr, len)) = control.get_addr() else {
            return; // skip virtual controls
        };

        let value = controller.get(name).unwrap();
        let value = control.value_to_buffer(value);

        let addr = addr as usize;
        match len {
            1 => {
                if value > u8::MAX as u32 {
                    warn!("Control {:?} value {} out of bounds!", name, value);
                }
                buffer[addr] = value as u8;
            }
            2 => {
                if value > u16::MAX as u32 {
                    warn!("Control {:?} value {} out of bounds!", name, value);
                }
                buffer[addr] = ((value >> 8) & 0xff) as u8;
                buffer[addr + 1] = (value & 0xff) as u8;
            }
            4 => {
                buffer[addr] = ((value >> 24) & 0xff) as u8;
                buffer[addr + 1] = ((value >> 16) & 0xff) as u8;
                buffer[addr + 2] = ((value >> 8) & 0xff) as u8;
                buffer[addr + 3] = (value & 0xff) as u8;
            }
            n => {
                error!("Control width {} not supported!", n)
            }
        }
    }
}