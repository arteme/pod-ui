use midir::*;
use anyhow::{Result, Context};
use std::sync::mpsc::Receiver;
use log::*;

use crate::midi::*;
use crate::config::PODS;
use crate::model::Config;

pub struct Midi {
    in_port: MidiInputPort,
    out_port: MidiOutputPort,

    conn_out: MidiOutputConnection,
    conn_in: MidiInputConnection<()>,

    rx: Receiver<(u64, Vec<u8>)>
}

impl Midi {
    pub fn new(in_port: Option<usize>, out_port: Option<usize>) -> Result<Self> {
        let mut midi_in = MidiInput::new("pod midi in")?;
        midi_in.ignore(Ignore::None);

        let midi_out = MidiOutput::new("pod midi out")?;

        let in_port_n: usize = in_port.unwrap_or(0);
        let in_port = midi_in.ports().into_iter().nth(in_port_n)
            .with_context(|| format!("MIDI input port {} not found", in_port_n))?;

        let out_port_n: usize = out_port.unwrap_or(0);
        let out_port = midi_out.ports().into_iter().nth(out_port_n)
            .with_context(|| format!("MIDI output port {} not found", out_port_n))?;

        let conn_out = midi_out.connect(&out_port, "pod midi out conn")
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        let (tx, rx) = std::sync::mpsc::channel();

        let conn_in = midi_in.connect(&in_port, "pod midi in conn", move |ts, data, _| {
            trace!("<< {}: {:02x?} len={}", ts, data, data.len());
            tx.send((ts, Vec::from(data))).unwrap();

        }, ())
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(Midi {
            in_port,
            out_port,
            conn_out,
            conn_in,
            rx
        })
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        trace!(">> {:02x?} len={}", bytes, bytes.len());
        self.conn_out.send(bytes)
            .map_err(|e| anyhow!("Midi send error: {:?}", e))
    }

    pub fn recv<T, F>(&mut self, callback: F) -> Result<T>
        where F: Fn(u64, Vec<u8>) -> Result<T>
    {
        self.rx.recv()
            .map_err(|e| anyhow!("Recv error: {}", e))
            .and_then(move |frame| callback(frame.0, frame.1))
    }
}

pub struct PodConfigs {
}

impl PodConfigs {
    pub fn new() -> Result<Self> {
        Ok(PodConfigs {})
    }

    pub fn count(&self) -> usize {
        PODS().len()
    }

    pub fn detect(&self, midi: &mut Midi) -> Result<&Config> {
        midi.send(MidiMessage::UniversalDeviceInquiry { channel: Channel::all() }.to_bytes().as_slice())?;
        midi.recv(move |_ts, data| {
            let event = MidiResponse::from_bytes(data)?;
            match event {
                MidiResponse::UniversalDeviceInquiry { channel: _, family, member, ver: _ } => {
                    let pod = PODS().iter().find(|config| {
                        family == config.family && member == config.member
                    }).unwrap();
                    info!("Discovered: {}", pod.name);
                    Ok(pod)
                }
                _ => Err(anyhow!("Incorrect MIDI response"))
            }
        })
    }

    pub fn dump_all(&self, midi: &mut Midi, config: &Config) -> Result<Vec<u8>> {
        midi.send(MidiMessage::AllProgramsDumpRequest.to_bytes().as_slice())?;
        midi.recv(move |_ts, data| {
            let event = MidiResponse::from_bytes(data)?;
            match event {
                MidiResponse::AllProgramsDump { ver: _, data } => {
                    if data.len() == config.all_programs_size {
                        Ok(data)
                    } else {
                        error!("Program size mismatch: expected {}, got {}", config.all_programs_size, data.len());
                        Err(anyhow!("Program size mismatch"))
                    }
                }
                _ => Err(anyhow!("Incorrect MIDI response"))
            }
        })
    }

    pub fn dump_edit(&self, midi: &mut Midi, config: &Config) -> Result<Vec<u8>> {
        midi.send(MidiMessage::ProgramEditBufferDumpRequest.to_bytes().as_slice())?;
        midi.recv(move |_ts, data| {
            let event = MidiResponse::from_bytes(data)?;
            match event {
                MidiResponse::ProgramEditBufferDump { ver: _, data } => {
                    if data.len() == config.program_size {
                        Ok(data)
                    } else {
                        error!("Program size mismatch: expected {}, got {}", config.program_size, data.len());
                        Err(anyhow!("Program size mismatch"))
                    }
                }
                _ => Err(anyhow!("Incorrect MIDI response"))
            }
        })
    }
}
