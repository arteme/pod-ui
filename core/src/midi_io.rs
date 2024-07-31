use midir::*;
use anyhow::*;
use regex::Regex;
use std::str::FromStr;
use std::time::Duration;
use async_stream::stream;
use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::time::sleep;
use log::*;
use result::prelude::*;

use crate::midi::*;
use crate::config::config_for_id;
use crate::model::Config;
use tokio::sync::mpsc;
use unicycle::IndexedStreamsUnordered;

#[async_trait]
pub trait MidiIn {
    fn name(&self) -> String;
    async fn recv(&mut self) -> Option<Vec<u8>>;
    fn close(&mut self);
}

#[async_trait]
pub trait MidiOut {
    fn name(&self) -> String;
    fn send(&mut self, bytes: &[u8]) -> Result<()>;
    fn close(&mut self);
}

pub type BoxedMidiIn = Box<dyn MidiIn + Send>;
pub type BoxedMidiOut = Box<dyn MidiOut + Send>;

pub fn box_midi_in<T: MidiIn + Send + 'static>(x: T) -> BoxedMidiIn {
    Box::new(x)
}

pub fn box_midi_out<T: MidiOut + Send + 'static>(x: T) -> BoxedMidiOut {
    Box::new(x)
}

pub struct MidiInPort {
    name: String,
    conn: Option<MidiInputConnection<()>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>
}

impl MidiInPort {
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
                .unwrap_or_else(|_| {
                    error!("midi input ({}): failed to send data to the application", n);
                });
        }, ())
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(MidiInPort { name, conn: Some(conn), rx })
    }
}

#[async_trait]
impl MidiIn for MidiInPort {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await
    }

    fn close(&mut self) {
        self.conn.take().map(|conn| {
            debug!("closing in");
            conn.close();
            debug!("closed in");
        });
        self.rx.close();
    }
}

impl Drop for MidiInPort {
    fn drop(&mut self) {
        self.close();
    }
}


pub struct MidiOutPort {
    name: String,
    conn: Option<MidiOutputConnection>,
}

impl MidiOutPort {
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

        Ok(MidiOutPort { name, conn: Some(conn) })
    }
}

#[async_trait]
impl MidiOut for MidiOutPort {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        trace!(">> {:02x?} len={}", bytes, bytes.len());
        if let Some(conn) = self.conn.as_mut() {
            conn.send(bytes)
                .map_err(|e| anyhow!("Midi send error: {:?}", e))
        } else {
            Err(anyhow!("Send error: connection already closed"))
        }
    }

    fn close(&mut self) {
        self.conn.take().map(|conn| {
            debug!("closing out");
            conn.close();
            debug!("closed out");
        });
    }
}

impl Drop for MidiOutPort {
    fn drop(&mut self) {
        self.close()
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

    fn new_for_address(port_addr: &str) -> Result<Self::Out> {
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

impl MidiOpen for MidiInPort {
    type Class = MidiInput;
    type Port = MidiInputPort;
    type Out = MidiInPort;
    const DIR: &'static str = "input";

    fn _new() -> Result<Self::Class> {
        MidiInPort::_new()
    }

    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out> {
        MidiInPort::_new_for_port(class, port)
    }
}

impl MidiOpen for MidiOutPort {
    type Class = MidiOutput;
    type Port = MidiOutputPort;
    type Out = MidiOutPort;
    const DIR: &'static str = "output";

    fn _new() -> Result<Self::Class> {
        MidiOutPort::_new()
    }

    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out> {
        MidiOutPort::_new_for_port(class, port)
    }
}


pub trait MidiPorts {
    fn all_ports() -> Result<Vec<String>>;
    fn ports() -> Result<Vec<String>>;
}

impl MidiPorts for MidiInPort {
    fn all_ports() -> Result<Vec<String>> {
        let midi = MidiInPort::_new()?;
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

impl MidiPorts for MidiOutPort {
    fn all_ports() -> Result<Vec<String>> {
        let midi = MidiOutPort::_new()?;
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

fn list_ports<T: MidiIO>(midi: T) -> Result<Vec<String>> {
    let port_names: Result<Vec<_>, _> =
        midi.ports().iter()
            .map(|port| midi.port_name(port))
            .collect::<Result<Vec<_>, _>>();
    port_names.map_err(|err| anyhow!("Error getting port names: {}", err))
}

const DETECT_DELAY: Duration = Duration::from_millis(1000);

async fn detect(in_ports: &mut [BoxedMidiIn], out_ports: &mut [BoxedMidiOut]) -> Result<(Vec<(usize, &'static Config)>, Option<String>)> {
    detect_with_channel(in_ports, out_ports, Channel::all()).await
}

async fn detect_with_channel(in_ports: &mut [BoxedMidiIn], out_ports: &mut [BoxedMidiOut], channel: u8) -> Result<(Vec<(usize, &'static Config)>, Option<String>)> {

    let in_names = in_ports.iter().map(|p| p.name()).collect::<Vec<_>>();
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
    let mut error: Option<String> = None;
    loop {
        tokio::select! {
            Some((i, Some(bytes))) = streams.next() => {
                if let Some(e) = check_for_broken_drivers(&in_names[i], &bytes) {
                    warn!("Detected broken drivers on port {:?}", &in_names[i]);
                    error.replace(e);
                }
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

    Ok((replied_midi_in, error))
}

async fn detect_channel(in_port: &mut BoxedMidiIn, out_port: &mut BoxedMidiOut) -> Result<Option<u8>> {

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

pub async fn autodetect(channel: Option<u8>) -> Result<(BoxedMidiIn, BoxedMidiOut, u8, &'static Config)> {

    let in_port_names = MidiInPort::ports()?;
    let mut in_port_errors = vec![];
    let in_ports = in_port_names.iter().enumerate()
        .flat_map(|(i, name)| {
            MidiInPort::new(Some(i)).map_err(|e| {
                let error = format!("Failed to open MIDI in port {:?}: {}", name, e);
                warn!("{}", error);
                in_port_errors.push(error);
            }).ok()
        })
        .map(box_midi_in)
        .collect::<Vec<_>>();

    let out_port_names = MidiOutPort::ports()?;
    let mut out_port_errors = vec![];
    let out_ports: Vec<BoxedMidiOut> = out_port_names.iter().enumerate()
        .flat_map(|(i, name)| {
            MidiOutPort::new(Some(i)).map_err(|e| {
                let error = format!("Failed to open MIDI out port {:?}: {}", name, e);
                warn!("{}", error);
                out_port_errors.push(error);
            }).ok()
        })
        .map(box_midi_out)
        .collect::<Vec<_>>();

    if in_ports.len() < 1 {
        if in_port_errors.len() < 1 {
            bail!("No MIDI input ports found")
        } else {
            bail!("Failed to open any MIDI input ports: {}", in_port_errors.join(", "))
        }
    }
    if out_ports.len() < 1 {
        if out_port_errors.len() < 1 {
            bail!("No MIDI output ports found")
        } else {
            bail!("Failed to open any MIDI output ports: {}", out_port_errors.join(", "))
        }
    }

    autodetect_with_ports(in_ports, out_ports, channel).await
}

pub async fn autodetect_with_ports(in_ports: Vec<BoxedMidiIn>, out_ports: Vec<BoxedMidiOut>,
                                   channel: Option<u8>) -> Result<(BoxedMidiIn, BoxedMidiOut, u8, &'static Config)> {
    let config: Option<&Config>;
    let mut in_ports = in_ports.into_iter().collect::<Vec<_>>();
    let mut out_ports = out_ports.into_iter().collect::<Vec<_>>();

    // 1. find the input
    {
        let (rep, error) = detect(in_ports.as_mut_slice(), out_ports.as_mut_slice()).await?;
        if rep.len() == 0 {
            if let Some(e) = error {
                bail!("{}", e);
            } else {
                bail!("Received no device response");
            }
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
        let mut error: Option<String> = None;
        let mut i = 0usize;
        for chunk in chunks {
            let (rep, e) = detect(in_ports.as_mut_slice(), chunk).await?;
            if let Some(e) = e {
                error.replace(e);
            }

            let rep = rep
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
            if let Some(e) = error {
                bail!("{}", e);
            } else {
                bail!("Received no device response (output search)");
            }
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
    let channel = match channel {
        None => {
            detect_channel(&mut in_port, &mut out_port).await?
        },
        Some(c) => {
            warn!("MIDI channel {} set manually", c);
            channel
        }
    };
    if channel.is_none() {
        bail!("Can't determine POD channel");
    }

    Ok((in_port, out_port, channel.unwrap(), config.unwrap()))
}

pub async fn test_with_ports(in_port: BoxedMidiIn, out_port: BoxedMidiOut, channel: u8, config: &Config) -> Result<(BoxedMidiIn, BoxedMidiOut, u8)> {
    let mut in_ports = vec![in_port];
    let mut out_ports = vec![out_port];

    let (rep, error) = detect_with_channel(
        in_ports.as_mut_slice(), out_ports.as_mut_slice(), channel
    ).await?;
    if rep.len() == 0 {
        if let Some(e) = error {
            bail!("{}", e);
        } else {
            bail!("Received no device response");
        }
    }
    if *rep[0].1 != *config {
        bail!("Incorrect device type");
    }

    Ok((in_ports.remove(0), out_ports.remove(0), channel))
}

pub async fn test(in_name: &str, out_name: &str, channel: u8, config: &Config) -> Result<(BoxedMidiIn, BoxedMidiOut, u8)> {
    let in_port = MidiInPort::new_for_name(in_name)?;
    let out_port = MidiOutPort::new_for_name(out_name)?;
    test_with_ports(box_midi_in(in_port), box_midi_out(out_port), channel, config).await
}

#[cfg(target_os = "linux")]
fn check_for_broken_drivers(port_name: &String, bytes: &Vec<u8>) -> Option<String> {
    if port_name.starts_with("PODxt") &&
        MidiMessage::from_bytes(bytes.clone()).ok().is_none() &&
        bytes.get(0) == Some(&0xf2) {

        let error = String::new() +
            "We've detected that you have a PODxt device connected via " +
            "USB. Unfortunately, your Linux kernel is old and contains " +
            "a broken PODxt driver. Please check <a href=\"https://github.com/arteme/pod-ui/issues/19\">" +
            "this tracking issue</a> for the kernel versions that have been " +
            "fixed and update your kernel accordingly. In the meantime, please " +
            "connect the device to a sound card using MIDI cables.";
        return Some(error)
    }

    None
}

#[cfg(not(target_os = "linux"))]
fn check_for_broken_drivers(_port_name: &String, _bytes: &Vec<u8>) -> Option<String> {
    None
}
