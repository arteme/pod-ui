use std::sync::{Arc, Mutex};
use anyhow::*;
use core::result::Result::Ok;
use log::*;
use result::*;
use pod_core::config::configs;
use pod_core::midi::Channel;
use pod_core::midi_io::*;
use pod_core::model::Config;
use pod_gtk::prelude::*;
use crate::opts::Opts;
use crate::{set_midi_in_out, State};

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

pub fn detect(state: Arc<Mutex<State>>, opts: Opts, window: &gtk::Window) -> Result<()> {

    // autodetect/open midi
    let autodetect = match (&opts.input, &opts.output) {
        (None, None) => true,
        _ => false
    };

    if autodetect {
        let state = state.clone();
        let window = window.clone();
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        tokio::spawn(async move {
            let res = pod_core::midi_io::autodetect().await;
            tx.send(res).ok();
        });
        rx.attach(None, move |autodetect| {
            match autodetect {
                Ok((midi_in, midi_out, midi_channel, config)) => {
                    set_midi_in_out(&mut state.lock().unwrap(),
                                    Some(midi_in), Some(midi_out), midi_channel, Some(config));
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
                                    None, None, Channel::all(), config);
                }
            };

            Continue(false)
        });
    } else {
        let midi_in =
            opts.input.map(MidiIn::new_for_address).invert()?;
        let midi_out =
            opts.output.map(MidiOut::new_for_address).invert()?;
        let midi_channel = opts.channel.unwrap_or(0);
        let config =
            opts.model.map(|str| config_for_str(&str)).invert()?;

        glib::idle_add_local_once(move || {
            set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out, midi_channel, config);
        });
    };

    Ok(())
}