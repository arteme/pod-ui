use midir::*;
use anyhow::{Result, Context};
use regex::Regex;
use std::str::FromStr;
use std::time::Duration;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::time::sleep;
use log::*;

use crate::midi::*;
use crate::config::configs;
use crate::model::Config;
use crate::util::OptionToResultsExt;
use tokio::sync::mpsc;

pub struct MidiIn {
    pub name: String,
    port: MidiInputPort,
    conn: MidiInputConnection<()>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>
}

impl MidiIn {
    fn _new() -> Result<MidiInput> {
        let mut midi_in = MidiInput::new("pod midi in")?;
        midi_in.ignore(Ignore::None);

        for (i, port) in midi_in.ports().iter().enumerate() {
            debug!("midi in {}: {:?}", i, midi_in.port_name(port)?);
        }

        Ok(midi_in)
    }

    pub fn _new_for_input(midi_in: MidiInput, in_port: Option<usize>) -> Result<Self> {
        let in_port_n: usize = in_port.unwrap_or(0);

        let port = midi_in.ports().into_iter().nth(in_port_n)
            .with_context(|| format!("MIDI input port {} not found", in_port_n))?;
        let name = midi_in.port_name(&port)
            .with_context(|| format!("Failed to get name for MIDI input port {}", in_port_n))?;

        let (tx, rx) = mpsc::unbounded_channel();

        let conn = midi_in.connect(&port, "pod midi in conn", move |ts, data, _| {
            trace!("<< {:02x?} len={} ts={}", data, data.len(), ts);
            tx.send(Vec::from(data)).unwrap();

        }, ())
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(MidiIn { name, port, conn, rx })
    }

    pub fn new(in_port: Option<usize>) -> Result<Self> {
        let midi_in = MidiIn::_new()?;
        MidiIn::_new_for_input(midi_in, in_port)
    }

    pub fn new_for_address(in_port: Option<String>) -> Result<Self> {
        let midi_in = MidiIn::_new()?;

        let n = in_port.and_then_r(|port| {
            let port_names: Result<Vec<_>, _> = midi_in.ports().iter()
                .map(|port| midi_in.port_name(port))
                .collect();

            find_address(port_names?.iter().map(String::as_str), &port)
        })?;

        MidiIn::_new_for_input(midi_in, n)
    }

    pub async fn recv(&mut self) -> Option<Vec<u8>>
    {
        self.rx.recv().await
    }
}


pub struct MidiOut {
    pub name: String,
    port: MidiOutputPort,
    conn: MidiOutputConnection,
}

impl MidiOut {
    fn _new() -> Result<MidiOutput> {
        let midi_out = MidiOutput::new("pod midi out")?;

        for (i, port) in midi_out.ports().iter().enumerate() {
            debug!("midi out {}: {:?}", i, midi_out.port_name(port)?);
        }

        Ok(midi_out)
    }

    fn _new_for_output(midi_out: MidiOutput, out_port: Option<usize>) -> Result<Self> {
        let out_port_n: usize = out_port.unwrap_or(0);
        let port = midi_out.ports().into_iter().nth(out_port_n)
            .with_context(|| format!("MIDI output port {} not found", out_port_n))?;
        let name = midi_out.port_name(&port)
            .with_context(|| format!("Failed to get name for MIDI output port {}", out_port_n))?;

        let conn = midi_out.connect(&port, "pod midi out conn")
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(MidiOut { name, port, conn })
    }

    pub fn new(out_port: Option<usize>) -> Result<Self> {
        let midi_out = MidiOut::_new()?;
        MidiOut::_new_for_output(midi_out, out_port)
    }

    pub fn new_for_address(out_port: Option<String>) -> Result<Self> {
        let out = MidiOut::_new()?;

        let n = out_port.and_then_r(|port| {
            let port_names: Result<Vec<_>, _> = out.ports().iter()
                .map(|port| out.port_name(port))
                .collect();

            find_address(port_names?.iter().map(String::as_str), &port)
        })?;

        MidiOut::_new_for_output(out, n)
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        trace!(">> {:02x?} len={}", bytes, bytes.len());
        self.conn.send(bytes)
            .map_err(|e| anyhow!("Midi send error: {:?}", e))
    }
}

trait MidiIO {
    fn ports() -> Result<Vec<String>>;
}

impl MidiIO for MidiIn {
    fn ports() -> Result<Vec<String>> {
        let midi = MidiIn::_new()?;
        list_ports(midi)
    }
}

impl MidiIO for MidiOut {
    fn ports() -> Result<Vec<String>> {
        let midi = MidiOut::_new()?;
        list_ports(midi)
    }
}

fn list_ports<T: midir::MidiIO>(midi: T) -> Result<Vec<String>> {
    let port_names: Result<Vec<_>, _> =
        midi.ports().iter()
            .map(|port| midi.port_name(port))
            .collect::<Result<Vec<_>, _>>();
    port_names.map_err(|err| anyhow!("Error getting port names: {}", err))
}

fn find_address<'a>(addresses: impl Iterator<Item = &'a str>, id: &'a str) -> Result<Option<usize>> {
    let port_n_re = Regex::new(r"\d+").unwrap();
    let port_id_re = Regex::new(r"\d+:\d+").unwrap();

    if port_id_re.is_match(id) {
        for (i, n) in addresses.enumerate() {
            if n.ends_with(id) {
                return Ok(Some(i));
            }
        }
        bail!("MIDI device with address {:?} not found", id);
    } else if port_n_re.is_match(id) {
        return Ok(Some(usize::from_str(id).unwrap()));
    }

    bail!("Failed to parse {:?} as a MIDI device address or index", id)
}
/*
pub struct PodConfigs {
}

impl PodConfigs {
    pub fn new() -> Result<Self> {
        Ok(PodConfigs {})
    }

    pub fn count(&self) -> usize {
        PODS.len()
    }

    pub fn by_name(&self, name: &String) -> Option<Config> {
        PODS.iter().find(|config| &config.name == name).map(|c| c.clone())
    }

    /*
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

     */
}

 */

const DETECT_DELAY: Duration = Duration::from_millis(1000);

async fn detect(in_ports: &mut [MidiIn], out_ports: &mut [MidiOut]) -> Result<Vec<usize>> {

    let udi = MidiMessage::UniversalDeviceInquiry { channel: Channel::all() }.to_bytes();

    let mut futures = FuturesUnordered::new();
    for (i, p) in in_ports.into_iter().enumerate() {
        futures.push(async move {
            p.rx.recv().await.map(|v| (i, v))
        })
    }
    let mut delay = Box::pin(sleep(DETECT_DELAY));

    for p in out_ports {
        p.send(&udi)?;
    }

    let mut replied_midi_in = Vec::<usize>::new();
    loop {
        tokio::select! {
            Some(Some((i, bytes))) = futures.next() => {
                let event = MidiMessage::from_bytes(bytes).ok();
                let found = match event {
                    Some(MidiMessage::UniversalDeviceInquiryResponse { family, member, .. }) => {
                        let pod: Option<&Config> = configs().iter().find(|config| {
                            family == config.family && member == config.member
                        });
                        pod.map(|pod| {
                            info!("Discovered: {}: {}", i, pod.name);
                            true
                        }).or_else(|| {
                            info!("Discovered unknown device: {}: {}/{}, skipping!", i, family, member);
                            Some(false)
                        }).unwrap()
                    },
                    _ => false
                };

                if found {
                    replied_midi_in.push(i);
                }
            },
            _ = &mut delay => { break; }
        }
    }

    Ok(replied_midi_in)
}

pub async fn autodetect() -> Result<(MidiIn, MidiOut)> {
    let in_port_names = MidiIn::ports()?;
    let mut in_ports = in_port_names.iter().enumerate()
        .filter(|(_, name)| !name.starts_with("pod midi out:"))
        .map(|(i, _)| MidiIn::new(Some(i)))
        .collect::<Result<Vec<_>>>()?;

    let out_port_names = MidiOut::ports()?;
    let mut out_ports = out_port_names.iter().enumerate()
        .filter(|(_, name)| !name.starts_with("pod midi in:"))
        .map(|(i, _)| MidiOut::new(Some(i)))
        .collect::<Result<Vec<_>>>()?;

    if in_ports.len() < 1 {
        bail!("No MIDI input ports found")
    }
    if out_ports.len() < 1 {
        bail!("No MIDI output ports found")
    }

    // 1. find the input
    {
        let rep = detect(in_ports.as_mut_slice(), out_ports.as_mut_slice()).await?;
        if rep.len() == 0 {
            bail!("Received no device response");
        }
        if rep.len() == in_ports.len() {
            bail!("Received device response on multiple ({}) ports", rep.len());
        }
        in_ports = in_ports.into_iter().enumerate()
            .filter(|(i, _)| *i == rep[0]).map(|(_,v)| v).collect();
    }

    // 2. find the output
    loop {
        let slice =  (out_ports.len() as f32 / 2.0).ceil() as usize;
        println!("len {} slice {}", out_ports.len(), slice);
        let chunks = out_ports.chunks_mut(slice);
        let mut good = Vec::<usize>::new();
        let mut i = 0usize;
        for chunk in chunks {
            let rep = detect(in_ports.as_mut_slice(), chunk).await?;
            if rep.len() > 0 {
                for x in i .. i+chunk.len() {
                    good.push(x);
                }
                // binary search: this group is good, let's continue with it!
                break;
            }
            i += chunk.len();
        }
        if good.len() == 0 {
            bail!("Received no device response (output search)");
        }
        if good.len() == out_ports.len() {
            bail!("Can't determine output port -- stuck at {}", good.len());
        }
        out_ports = out_ports.into_iter().enumerate()
            .filter(|(i, _)| good.contains(i)).map(|(_,v)| v).collect();
        if out_ports.len() == 1 {
            break;
        }
    }

    Ok((in_ports.remove(0), out_ports.remove(0)))
}