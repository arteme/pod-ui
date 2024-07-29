use clap::Parser;
use anyhow::Result;
use std::fmt::Write;
use pod_core::config::configs;
use pod_core::midi_io::{MidiInPort, MidiOutPort, MidiPorts};
use crate::usb::*;

#[derive(Parser, Clone)]
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
    /// Select the MIDI channel the POD is configured on. 0 means "omni" mode,
    /// i.e. it will listen on all channels simultaneously, values 1 - 16
    /// configure specific channel. This option also sets which MIDI channel
    /// the pod-ui application will listen on.
    /// This setting may not be relevant for all different devices supported.
    pub channel: Option<u8>,

    /// Select the USB device to be connected as MIDI input/output. <USB> must be
    /// an integer index of a recognized USB device present on this system. This
    /// can also be an <bus>:<address> pair, such as "5:8".
    /// When `-u` is provided, neither `-i` nor `-o` can be provided, or an error
    /// will be reported.
    #[cfg(feature = "usb")]
    #[clap(short, long)]
    pub usb: Option<String>,

    #[cfg(not(feature = "usb"))]
    pub usb: Option<String>,

    #[clap(short, long)]
    /// Select the model of the device. <MODEL> must be either an
    /// integer index of a supported device model or a string name
    /// of the model in question. Only used when both `-i` and `-o`
    /// are given. If `-i` and `-o` options are given, but `-m` is
    /// omitted, the device model on specified ports will be detected.
    pub model: Option<String>,

    #[clap(short, long)]
    /// Run a stand-alone instance of the pod-ui GTK application
    /// instead of triggering any events on an already-running
    /// pod-ui application.
    pub standalone: bool,
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
    for (i, n) in MidiInPort::ports().ok().unwrap_or_default().iter().enumerate() {
        writeln!(s, "{}[{}] {}", tab, i, n)?;
    }
    writeln!(s, "")?;
    writeln!(s, "MIDI output ports (-o):")?;
    for (i, n) in MidiOutPort::ports().ok().unwrap_or_default().iter().enumerate() {
        writeln!(s, "{}[{}] {}", tab, i, n)?;
    }
    writeln!(s, "")?;

    if cfg!(feature = "usb") {
        writeln!(s, "USB devices (-u):")?;
        for (i, n) in usb_list_devices().iter().enumerate() {
            writeln!(s, "{}[{}] {}", tab, i, n)?;
        }
        writeln!(s, "")?;
    }

    Ok(s)
}
