use std::sync::{Arc, Mutex};
use anyhow::*;
use core::result::Result::Ok;
use log::*;
use pod_core::config::configs;
use pod_core::midi::Channel;
use pod_core::midi_io::*;
use pod_core::model::Config;
use pod_gtk::prelude::*;
use crate::opts::Opts;
use crate::{set_midi_in_out, State};
use crate::usb::{usb_open_addr, usb_open_name};

fn config_for_str(config_str: &str) -> Result<&'static Config> {
    use std::str::FromStr;
    use regex::Regex;

    let n_re = Regex::new(r"\d+").unwrap();

    let mut found = None;
    if n_re.is_match(&config_str) {
        let index = usize::from_str(&config_str)
            .with_context(|| format!("Unrecognized config index {:?}", config_str))?;
        let config = configs().get(index)
            .with_context(|| format!("Config with index {} not found!", index))?;
        found = Some(config);
    } else {
        for c in configs().iter() {
            if c.name.eq_ignore_ascii_case(&config_str) {
                found = Some(c);
                break;
            }
        }
        if found.is_none() {
            bail!("Config with name {:?} not found!", config_str);
        }
    }

    Ok(found.unwrap())
}

pub fn detect(state: Arc<Mutex<State>>, opts: Opts, window: &gtk::Window) -> Result<()>
{
    let mut ports: Option<(BoxedMidiIn, BoxedMidiOut)> = None;
    let mut config = None;

    // autodetect/open midi
    let autodetect = match (&opts.input, &opts.output, &opts.usb, &opts.model) {
        (None, None, None, None) => true,
        (None, None, None, Some(_)) => {
            warn!("Model set on command line, but not input/output ports. \
                   The model parameter will be ignored!");
            true
        }
        (Some(_), None, None, _) | (None, Some(_), None, _) => {
            bail!("Both input and output port need to be set on command line to skip autodetect!")
        }
        (Some(_), _, Some(_), _) | (_, Some(_), Some(_), _) => {
            bail!("MIDI and USB inputs cannot be set on command line together, use either MIDI or USB!")
        }
        // MIDI
        (Some(i), Some(o), None, None) => {
            let midi_in = MidiInPort::new_for_address(i)?;
            let midi_out = MidiOutPort::new_for_address(o)?;
            ports = Some((Box::new(midi_in), Box::new(midi_out)));
            true
        }
        (Some(i), Some(o), None, Some(m)) => {
            let midi_in = MidiInPort::new_for_address(i)?;
            let midi_out = MidiOutPort::new_for_address(o)?;
            ports = Some((Box::new(midi_in), Box::new(midi_out)));
            config = Some(config_for_str(m)?);
            false
        }
        // USB
        (None, None, Some(u), None) => {
            let (midi_in, midi_out) = usb_open_addr(u)?;
            ports = Some((Box::new(midi_in), Box::new(midi_out)));
            true
        }
        (None, None, Some(u), Some(m)) => {
            let (midi_in, midi_out) = usb_open_addr(u)?;
            ports = Some((Box::new(midi_in), Box::new(midi_out)));
            config = Some(config_for_str(m)?);
            false
        }
    };
    let midi_channel = match opts.channel {
        None  => None,
        Some(x) if x == 0 => Some(Channel::all()),
        Some(x) if (1u8 ..= 16).contains(&x) => Some(x - 1),
        Some(x) => {
            bail!("Midi channel {} out of bounds (0, 1..16)", x);
        }
    };
    // channel, when not auto-detected
    let midi_channel_u8 = midi_channel.unwrap_or(Channel::all());

    let state = state.clone();
    let window = window.clone();
    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    if autodetect {
        tokio::spawn(async move {
            let res = if let Some((midi_in, midi_out)) = ports {
                // autodetect device on provided ports
                pod_core::midi_io::autodetect_with_ports(
                    vec![midi_in], vec![midi_out], midi_channel
                ).await
            } else {
                // autodetect device
                pod_core::midi_io::autodetect(midi_channel).await
            };
            let res = res.and_then(|(midi_in, midi_out, chan, config)|
                Ok((midi_in, midi_out, chan, false, config)));
            tx.send(res).ok();
        });
    } else {
        // manually configured device
        let (midi_in, midi_out) = ports.unwrap();
        tx.send(Ok((midi_in, midi_out, midi_channel_u8, false, config.unwrap()))).ok();
    }

    rx.attach(None, move |autodetect| {
        match autodetect {
            Ok((midi_in, midi_out, midi_channel, is_usb, config)) => {
                set_midi_in_out(&mut state.lock().unwrap(),
                                Some(midi_in), Some(midi_out), midi_channel, is_usb, Some(config));
            }
            Err(e) => {
                error!("MIDI autodetect failed: {}", e);

                if e.to_string().starts_with("We've detected that you have a PODxt") {
                    let m = gtk::MessageDialog::new(
                        Some(&window),
                        gtk::DialogFlags::DESTROY_WITH_PARENT,
                        gtk::MessageType::Error,
                        gtk::ButtonsType::Ok,
                        "Autodetect encountered errors:"
                    );
                    m.set_secondary_use_markup(true);
                    m.set_secondary_text(Some(e.to_string().as_str()));
                    m.connect_response(|dialog, _| {
                        dialog.close();
                    });
                    m.show();
                }

                let config = opts.model.as_ref()
                    .and_then(|str| config_for_str(&str).ok())
                    .or_else(|| configs().iter().next());
                set_midi_in_out(&mut state.lock().unwrap(),
                                None, None, midi_channel_u8, false, config);
            }
        };

        Continue(false)
    });

    Ok(())
}

pub async fn test(in_name: &str, out_name: &str, channel: u8, is_usb: bool, config: &Config) -> Result<(BoxedMidiIn, BoxedMidiOut, u8)> {
    let (midi_in, midi_out) = open(in_name, out_name, is_usb)?;
    test_with_ports(midi_in, midi_out, channel, config).await
}

pub fn open(in_name: &str, out_name: &str, is_usb: bool) -> Result<(BoxedMidiIn, BoxedMidiOut)> {
    let res = if is_usb {
        if in_name != out_name {
            bail!("USB device input/output names do not match");
        }
        let (midi_in, midi_out) = usb_open_name(in_name)?;
        (box_midi_in(midi_in), box_midi_out(midi_out))
    } else {
        let midi_in = MidiInPort::new_for_name(in_name)?;
        let midi_out = MidiOutPort::new_for_name(out_name)?;
        (box_midi_in(midi_in), box_midi_out(midi_out))
    };
    Ok(res)
}
