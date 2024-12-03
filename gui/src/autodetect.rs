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
use crate::{set_midi_in_out, State, usb};

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
    let mut midi_channel: Option<u8> = None;
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
            let (midi_in, midi_out) = usb::usb_open_addr(u)?;
            ports = Some((Box::new(midi_in), Box::new(midi_out)));
            midi_channel = Some(Channel::num(0)); // USB devices don't care about the MIDI channel
            true
        }
        (None, None, Some(u), Some(m)) => {
            let (midi_in, midi_out) = usb::usb_open_addr(u)?;
            ports = Some((Box::new(midi_in), Box::new(midi_out)));
            config = Some(config_for_str(m)?);
            false
        }
    };
    let midi_channel = match opts.channel {
        None => midi_channel, // use channel default of None, unless set by the logic above
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
    let (tx, rx) = glib::MainContext::channel(glib::Priority::DEFAULT);

    if autodetect {
        tokio::spawn(async move {
            let res = if let Some((midi_in, midi_out)) = ports {
                // autodetect device on provided ports
                pod_core::midi_io::autodetect_with_ports(
                    vec![midi_in], vec![midi_out], midi_channel
                ).await
            } else {
                // autodetect device
                run_autodetect(midi_channel).await
            };
            let res = res.and_then(|res|
                Ok((res.in_port, res.out_port, res.channel, false, res.config)));
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

        ControlFlow::Break
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
        let (midi_in, midi_out) = usb::usb_open_name(in_name)?;
        (box_midi_in(midi_in), box_midi_out(midi_out))
    } else {
        let midi_in = MidiInPort::new_for_name(in_name)?;
        let midi_out = MidiOutPort::new_for_name(out_name)?;
        (box_midi_in(midi_in), box_midi_out(midi_out))
    };
    Ok(res)
}

/**
 * Run MIDI device auto-detect.
 *
 * Run MIDI-specific device auto-detect and, if failed, USB-specific
 * device auto-detect. The latter ignores the MIDI channel that may
 * have been previously set.
 */
pub async fn run_autodetect(channel: Option<u8>) -> Result<AutodetectResult> {
    match autodetect(channel).await {
        res @ Ok(_) => res,
        Err(e) => {
            if !usb::autodetect_supported() {
                return Err(e);
            }

            match usb::autodetect().await {
                res @ Ok(_) => res,
                Err(e1) => {
                   // do something clever with the replies here
                    bail!("MIDI: {}\nUSB: {}\n", e.to_string(), e1.to_string())
                }
            }
        }
    }
}
