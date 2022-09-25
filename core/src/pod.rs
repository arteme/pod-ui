use midir::*;
use anyhow::{Result, Context};
use regex::Regex;
use std::str::FromStr;
use std::time::Duration;
use async_stream::stream;
use futures_util::StreamExt;
use tokio::time::sleep;
use log::*;
use result::prelude::*;

use crate::midi::*;
use crate::config::config_for_id;
use crate::model::Config;
use tokio::sync::mpsc;
use unicycle::IndexedStreamsUnordered;

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

    pub fn _new_for_port(midi_in: MidiInput, port: MidiInputPort) -> Result<Self> {
        let name = midi_in.port_name(&port)
            .map_err(|e| anyhow!("Failed to get MIDI input port name: {}", e))?;

        let (tx, rx) = mpsc::unbounded_channel();

        let n = name.clone();
        let conn = midi_in.connect(&port, "pod midi in conn", move |ts, data, _| {
            trace!("<< {:02x?} len={} ts={}", data, data.len(), ts);
            tx.send(Vec::from(data))
                .unwrap_or_else(|e| {
                    error!("midi input ({}): failed to send data to the application", n);
                });

        }, ())
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(MidiIn { name, port, conn, rx })
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

    fn _new_for_port(midi_out: MidiOutput, port: MidiOutputPort) -> Result<Self> {
        let name = midi_out.port_name(&port)
            .map_err(|e| anyhow!("Failed to get MIDI output port name: {}", e))?;
        let conn = midi_out.connect(&port, "pod midi out conn")
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(MidiOut { name, port, conn })
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        trace!(">> {:02x?} len={}", bytes, bytes.len());
        self.conn.send(bytes)
            .map_err(|e| anyhow!("Midi send error: {:?}", e))
    }
}

pub trait  MidiOpen {
    type Class: MidiIO<Port = Self::Port>;
    type Port;
    type Out;
    const DIR: &'static str;

    fn _new() -> Result<Self::Class>;
    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out>;

    fn new(port_idx: Option<usize>) -> Result<Self::Out> {
        let class = Self::_new()?;

        let port_n: usize = port_idx.unwrap_or(0);
        let port = class.ports().into_iter().nth(port_n)
            .with_context(|| format!("MIDI {} port {} not found", Self::DIR, port_n))?;

        Self::_new_for_port(class, port)
    }

    fn new_for_address(port_addr: String) -> Result<Self::Out> {
        let class = Self::_new()?;

        let port_n_re = Regex::new(r"\d+").unwrap();
        let port_id_re = Regex::new(r"\d+:\d+").unwrap();

        let mut found = None;
        if port_id_re.is_match(&port_addr) {
            for port in class.ports().into_iter() {
                let name = class.port_name(&port)?;
                if name.ends_with(&port_addr) {
                    found = Some(port);
                }
            }
        } else if port_n_re.is_match(&port_addr) {
            let n = Some(usize::from_str(&port_addr)).invert()
                .with_context(|| format!("Unrecognized MIDI port index {:?}", port_addr))?;
            return Self::new(n);
        } else {
            bail!("Unrecognized MIDI port address {:?}", port_addr);
        }

        if found.is_none() {
            bail!("MIDI {} port for address {:?} not found!", Self::DIR, port_addr);
        }

        Self::_new_for_port(class, found.unwrap())
    }

    fn new_for_name(port_name: &str) -> Result<Self::Out> {
        let class = Self::_new()?;

        let mut found = None;
        for port in class.ports().into_iter() {
            let name = class.port_name(&port)?;
            if name == port_name {
                found = Some(port);
            }
        }
        if found.is_none() {
            bail!("MIDI {} port for name {:?} not found!", Self::DIR, port_name);
        }

        Self::_new_for_port(class, found.unwrap())
    }
}

impl MidiOpen for MidiIn {
    type Class = MidiInput;
    type Port = MidiInputPort;
    type Out = MidiIn;
    const DIR: &'static str = "input";

    fn _new() -> Result<Self::Class> {
        MidiIn::_new()
    }

    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out> {
        MidiIn::_new_for_port(class, port)
    }
}

impl MidiOpen for MidiOut {
    type Class = MidiOutput;
    type Port = MidiOutputPort;
    type Out = MidiOut;
    const DIR: &'static str = "output";

    fn _new() -> Result<Self::Class> {
        MidiOut::_new()
    }

    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out> {
        MidiOut::_new_for_port(class, port)
    }
}


pub trait MidiPorts {
    fn all_ports() -> Result<Vec<String>>;
    fn ports() -> Result<Vec<String>>;
}

impl MidiPorts for MidiIn {
    fn all_ports() -> Result<Vec<String>> {
        let midi = MidiIn::_new()?;
        list_ports(midi)
    }

    fn ports() -> Result<Vec<String>> {
        Self::all_ports()
            .map(|v| v.into_iter()
                .filter(|name| !name.starts_with("pod midi out:"))
                .collect()
            )
    }
}

impl MidiPorts for MidiOut {
    fn all_ports() -> Result<Vec<String>> {
        let midi = MidiOut::_new()?;
        list_ports(midi)
    }

    fn ports() -> Result<Vec<String>> {
        Self::all_ports()
            .map(|v| v.into_iter()
                .filter(|name| !name.starts_with("pod midi in:"))
                .collect()
            )
    }
}

fn list_ports<T: midir::MidiIO>(midi: T) -> Result<Vec<String>> {
    let port_names: Result<Vec<_>, _> =
        midi.ports().iter()
            .map(|port| midi.port_name(port))
            .collect::<Result<Vec<_>, _>>();
    port_names.map_err(|err| anyhow!("Error getting port names: {}", err))
}

const DETECT_DELAY: Duration = Duration::from_millis(1000);

async fn detect(in_ports: &mut [MidiIn], out_ports: &mut [MidiOut]) -> Result<Vec<(usize, &'static Config)>> {
    detect_with_channel(in_ports, out_ports, Channel::all()).await
}

async fn detect_with_channel(in_ports: &mut [MidiIn], out_ports: &mut [MidiOut], channel: u8) -> Result<Vec<(usize, &'static Config)>> {

    let udi = MidiMessage::UniversalDeviceInquiry { channel }.to_bytes();

    let mut streams = IndexedStreamsUnordered::new();
    for p in in_ports.iter_mut() {
        let s = stream! {
          while let Some(data) = p.recv().await {
                yield data;
            }
        };
        streams.push(s);
    }
    let mut delay = Box::pin(sleep(DETECT_DELAY));

    for p in out_ports {
        p.send(&udi)?;
    }

    let mut replied_midi_in = Vec::<(usize, &Config)>::new();
    loop {
        tokio::select! {
            Some((i, Some(bytes))) = streams.next() => {
                let event = MidiMessage::from_bytes(bytes).ok();
                let found = match event {
                    Some(MidiMessage::UniversalDeviceInquiryResponse { family, member, .. }) => {
                        let pod = config_for_id(family, member);
                        pod.map(|pod| {
                            info!("Discovered: {}: {}", i, pod.name);
                            pod
                        }).or_else(|| {
                            info!("Discovered unknown device: {}: {}/{}, skipping!", i, family, member);
                            None
                        })
                    },
                    _ => None
                };

                if let Some(config) = found {
                    replied_midi_in.push((i, config));
                }
            },
            _ = &mut delay => { break; }
        }
    }

    Ok(replied_midi_in)
}

async fn detect_channel(in_port: &mut MidiIn, out_port: &mut MidiOut) -> Result<Option<u8>> {

    let udi = (0u8..=15).into_iter().map(|n| {
        MidiMessage::UniversalDeviceInquiry { channel: Channel::num(n) }.to_bytes()
    }).chain(std::iter::once(
        MidiMessage::UniversalDeviceInquiry { channel: Channel::all() }.to_bytes()
    ));

    let input = stream! {
      while let Some(data) = in_port.recv().await {
            yield data;
        }
    };
    let mut input = Box::pin(input);
    let mut delay = Box::pin(sleep(DETECT_DELAY));

    for msg in udi {
        out_port.send(&msg)?;
    }

    let mut channel: Option<u8> = None;
    loop {
        tokio::select! {
            Some(bytes) = input.next() => {
                let event = MidiMessage::from_bytes(bytes).ok();
                let found = match event {
                    Some(MidiMessage::UniversalDeviceInquiryResponse { family, member, channel, .. }) => {
                        let pod = config_for_id(family, member);
                        pod.map(|pod| {
                            info!("Discovered: channel={}: {}", channel, pod.name);
                            channel
                        }).or_else(|| {
                            info!("Discovered unknown device: channel={}: {}/{}, skipping!",
                                channel, family, member);
                            None
                        })
                    },
                    _ => None
                };

                if found.is_some() {
                    channel = found;
                    break;
                }
            },
            _ = &mut delay => { break; }
        }
    }

    Ok(channel)
}

pub async fn autodetect() -> Result<(MidiIn, MidiOut, u8, &'static Config)> {
    let in_port_names = MidiIn::ports()?;
    let mut in_ports = in_port_names.iter().enumerate()
        .map(|(i, _)| MidiIn::new(Some(i)))
        .collect::<Result<Vec<_>>>()?;

    let out_port_names = MidiOut::ports()?;
    let mut out_ports = out_port_names.iter().enumerate()
        .map(|(i, _)| MidiOut::new(Some(i)))
        .collect::<Result<Vec<_>>>()?;

    let mut config: Option<&Config> = None;

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
        if rep.len() > 1 {
            bail!("Received device response on multiple ({}) ports", rep.len());
        }
        in_ports = in_ports.into_iter().enumerate()
            .filter(|(i, _)| *i == rep[0].0).map(|(_,v)| v).collect();
        config = Some(rep[0].1);
    }

    // 2. find the output
    loop {
        let slice =  (out_ports.len() as f32 / 2.0).ceil() as usize;
        let chunks = out_ports.chunks_mut(slice);
        let mut good = Vec::<usize>::new();
        let mut i = 0usize;
        for chunk in chunks {
            let rep = detect(in_ports.as_mut_slice(), chunk).await?
                .into_iter()
                // make sure we only count the ports that have the same device as in step 1
                .filter(|(_, c)| config.filter(|c1| *c1 == *c).is_some())
                .collect::<Vec<_>>();
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
        if good.len() == out_ports.len() && good.len() > 1 {
            bail!("Can't determine output port -- stuck at {}", good.len());
        }
        out_ports = out_ports.into_iter().enumerate()
            .filter(|(i, _)| good.contains(i)).map(|(_,v)| v).collect();
        if out_ports.len() == 1 {
            break;
        }
    }

    // 3. find the channel
    let mut in_port = in_ports.remove(0);
    let mut out_port = out_ports.remove(0);
    let channel = detect_channel(&mut in_port, &mut out_port).await?;
    if channel.is_none() {
        bail!("Can't determine POD channel");
    }

    Ok((in_port, out_port, channel.unwrap(), config.unwrap()))
}

pub async fn test(in_name: &str, out_name: &str, channel: u8, config: &Config) -> Result<(MidiIn, MidiOut, u8)> {
    let in_port = MidiIn::new_for_name(in_name)?;
    let out_port = MidiOut::new_for_name(out_name)?;
    let mut in_ports = vec![in_port];
    let mut out_ports = vec![out_port];

    let rep = detect_with_channel(
        in_ports.as_mut_slice(), out_ports.as_mut_slice(), channel
    ).await?;
    if rep.len() == 0 {
        bail!("Received no device response");
    }
    if *rep[0].1 != *config {
        bail!("Incorrect device type");
    }

    Ok((in_ports.remove(0), out_ports.remove(0), channel))
}
