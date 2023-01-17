use clap::Parser;
use anyhow::Result;
use std::fmt::Write;
use pod_core::config::configs;
use pod_core::midi_io::{MidiIn, MidiOut, MidiPorts};

#[derive(Parser)]
pub struct Opts {
    #[clap(short, long)]
    /// Select the MIDI port to be connected as input. <INPUT> must be an
    /// integer index of a MIDI input port present on this system. On Linux,
    /// this can also be an ALSA <client>:<port> pair, such as "20:0".
    /// To select ports manually, both `-i` and `-o` options must be provided.
    /// If both `-i` and `-o` are provided, port autodetect will be skipped.
    /// If only `-i` is provided, an error will be reported.
    pub input: Option<String>,

    #[clap(short, long)]
    /// Select the MIDI port to be connected as output. <OUTPUT> must be an
    /// integer index of a MIDI output port present on this system. On Linux,
    /// this can also be an ALSA <client>:<port> pair, such as "20:0".
    /// If both `-i` and `-o` are provided, port autodetect will be skipped.
    /// If only `-o` is provided, an error will be reported.
    pub output: Option<String>,

    #[clap(short, long)]
    /// Select the MIDI channel the POD is configured on. 0 means "all",
    /// values 1 - 16 configure individual channels. This option also affects
    /// which MIDI channel the pod-ui application will listen on.
    /// This setting may not be relevant for all different devices supported.
    pub channel: Option<u8>,

    #[clap(short, long)]
    /// Select the model of the device. <MODEL> must be either an
    /// integer index of a supported device model or a string name
    /// of the model in question. Only used when both `-i` and `-o`
    /// are given. If `-i` and `-o` options are given, but `-m` is
    /// omitted, the device model on specified ports will be detected.
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
