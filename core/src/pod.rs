use midir::*;
use anyhow::{Result, Context, Error};
use hocon::HoconLoader;
use serde::Deserialize;
use std::env;
use log::*;

use crate::model::Config;
use crate::midi::*;
use std::path::Path;
use std::sync::mpsc::Receiver;

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
            trace!("<< {}: {:02x?}", ts, data);
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
        trace!(">> {:02x?}", bytes);
        self.conn_out.send(bytes)
            .map_err(|e| anyhow!("Midi send error: {:?}", e))
    }

    pub fn recv<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(u64, Vec<u8>) -> Result<()>
    {
        self.rx.recv()
            .map_err(|e| anyhow!("Recv error: {}", e))
            .and_then(move |frame| callback(frame.0, frame.1))
    }
}

#[derive(Deserialize, Debug)]
pub struct PodConfigs {
    configs: Vec<Config>
}

impl PodConfigs {
    pub fn new() -> Result<Self> {

        fn find_pods_conf(root: &str) -> Result<String> {
            let path = format!("{}/pods.conf", root);
            let file = Path::new(path.as_str());
            if !file.is_file() {
                warn!("Config file not found: {:?}", file);
                return Err(Error::msg("not found"));
            }
            info!("Config file found: {:?}", file);
            Ok(path)
        }

        let path = Err(Error::msg("no paths checked"))
            .or_else(|_| find_pods_conf("."))
            .or_else(|_|
                env::var("POD_CONFIG_PATH").map_err(|_| Error::msg("env var not set"))
                .and_then(|var| find_pods_conf(var.as_str())))
            .context("Pods configuration file 'pods.conf' not found")?;

        let loader = HoconLoader::new().load_file(path)?;
        info!("{:?}", loader);
        let configs: PodConfigs = loader.resolve()?;
        Ok(configs)
    }

    pub fn count(&self) -> usize {
        self.configs.len()
    }

    pub fn detect(&self, midi: &mut Midi) -> Result<()> {
        midi.send(MidiMessage::UniversalDeviceInquiry { channel: Channel::all() }.to_bytes().as_slice())?;

        loop {
            midi.recv(move |ts, data| {
                let event = MidiResponse::from_bytes(data);
                trace!("-- {}: {:?}", ts, event);

                event.map(|event| {
                    match event {
                        MidiResponse::UniversalDeviceInquiry { channel, family, member, ver } => {
                            self.configs.iter().find(|config| {
                                family == config.family && member == config.member
                            })
                                .unwrap()
                        }
                    }
                })?;

                Ok(())
            })?;

        }

        Ok(())
    }
}
