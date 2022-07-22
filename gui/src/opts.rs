use clap::Parser;
use anyhow::Result;
use std::fmt::Write;
use pod_core::config::configs;
use pod_core::pod::{MidiIn, MidiOut, MidiPorts};

#[derive(Parser)]
pub struct Opts {
    #[clap(short, long)]
    /// Select the MIDI port to be connected as input. <INPUT> must be an
    /// integer index of a MIDI input port present on this system. On Linux,
    /// this can also be an ALSA <client>:<port> pair, such as "20:0".
    /// If either `-i` or `-o` option is omitted, the MIDI input/output port
    /// pair will be auto-detected.
    pub input: Option<String>,

    #[clap(short, long)]
    /// Select the MIDI port to be connected as output. <OUTPUT> must be an
    /// integer index of a MIDI output port present on this system. On Linux,
    /// this can also be an ALSA <client>:<port> pair, such as "20:0".
    /// If either `-i` or `-o` option is omitted, the MIDI input/output port
    /// pair will be auto-detected.
    pub output: Option<String>,

    #[clap(short, long)]
    /// Select the MIDI channel the POD is configured on. 0 means "all",
    /// values 1 - 15 configure individual channels. If either `-i` or `-o`
    /// is omitted, the MIDI channel will be auto-detected. This setting
    /// may not be relevant for all different devices supported.
    pub channel: Option<u8>,

    #[clap(short, long)]
    /// Select the model of the device. <MODEL> must be either an
    /// integer index of a supported device model or a string name
    /// of the model in question. Only used when both `-i` and `-o`
    /// are given.
    pub model: Option<String>,
}

pub fn generate_help_text() -> Result<String> {
    let mut s = String::new();
    let tab = "    ";

    writeln!(s, "Device models (-m):")?;
    for (i, c) in configs().iter().enumerate() {
        writeln!(s, "{}[{}] {}", tab, i, &c.name)?;
    }
    writeln!(s, "")?;
    writeln!(s, "MIDI input ports (-i):")?;
    for (i, n) in MidiIn::ports().ok().unwrap_or_default().iter().enumerate() {
        writeln!(s, "{}[{}] {}", tab, i, n)?;
    }
    writeln!(s, "")?;
    writeln!(s, "MIDI output ports (-o):")?;
    for (i, n) in MidiOut::ports().ok().unwrap_or_default().iter().enumerate() {
        writeln!(s, "{}[{}] {}", tab, i, n)?;
    }
    writeln!(s, "")?;

    Ok(s)
}
