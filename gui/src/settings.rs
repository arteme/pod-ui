use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use pod_core::midi_io::*;
use pod_gtk::prelude::*;
use gtk::{IconSize, ResponseType};
use crate::{gtk, midi_in_out_start, midi_in_out_stop, set_midi_in_out, State};
use crate::util::ManualPoll;

use log::*;
use pod_core::config::configs;
use pod_core::midi::Channel;
use pod_core::model::MidiQuirks;

#[derive(Clone)]
struct SettingsDialog {
    dialog: gtk::Dialog,
    midi_in_combo: gtk::ComboBoxText,
    midi_out_combo: gtk::ComboBoxText,
    midi_channel_combo: gtk::ComboBoxText,
    model_combo: gtk::ComboBoxText,
    autodetect_button: gtk::Button,
    test_button: gtk::Button,
    message_label: gtk::Label,
    message_image: gtk::Image,

    spinner: Option<gtk::Spinner>
}

impl SettingsDialog {
    fn new(ui: &gtk::Builder) -> Self {
        SettingsDialog {
            dialog: ui.object("settings_dialog").unwrap(),
            midi_in_combo: ui.object("settings_midi_in_combo").unwrap(),
            midi_out_combo: ui.object("settings_midi_out_combo").unwrap(),
            midi_channel_combo: ui.object("settings_midi_channel_combo").unwrap(),
            model_combo: ui.object("settings_model_combo").unwrap(),
            autodetect_button: ui.object("settings_autodetect_button").unwrap(),
            test_button: ui.object("settings_test_button").unwrap(),
            message_label: ui.object("settings_message_label").unwrap(),
            message_image: ui.object("settings_message_image").unwrap(),
            spinner: None
        }
    }

    fn set_interactive(&self, sensitive: bool) {
        self.dialog.set_response_sensitive(ResponseType::Ok, sensitive);
        self.midi_in_combo.set_sensitive(sensitive);
        self.midi_out_combo.set_sensitive(sensitive);
        self.midi_channel_combo.set_sensitive(sensitive);
        self.model_combo.set_sensitive(sensitive);
        self.autodetect_button.set_sensitive(sensitive);
        self.test_button.set_sensitive(sensitive);
    }

    fn set_message(&self, icon: &str, message: &str) {
        let icon = if icon.is_empty() { None } else { Some(icon) };
        self.message_image.set_from_icon_name(icon, IconSize::Dialog);
        self.message_label.set_label(message);
    }

    fn clear_message(&self) {
        self.set_message("", "");
    }

    fn work_start(&mut self, button: Option<&gtk::Button>) {
        if let Some(button) = button {
            let spinner = gtk::Spinner::new();
            (*button).set_image(Some(&spinner));
            spinner.start();

            self.spinner = Some(spinner);
        }

        self.clear_message();
        self.set_interactive(false);
    }

    fn work_finish(&mut self, icon: &str, message: &str) {
        self.set_message(icon, message);
        self.set_interactive(true);

        if let Some(spinner) = self.spinner.take() {
            spinner.stop();
        }
    }
}

static CHANNELS: &'static [&str]  = &[
    "1", "2", "3", "4", "5", "6", "7", "8", "9",
    "10", "11", "12", "13", "14", "15", "16", "Omni mode"
];

fn midi_channel_to_combo_index(channel: u8) -> Option<u32> {
    match channel {
        x if x == Channel::all() => Some(16),
        x => Some(x as u32)
    }
}

fn midi_channel_from_combo_index(index: Option<u32>) -> u8 {
    match index {
        Some(16) => Channel::all(),
        Some(x) => x as u8,
        None => 0
    }
}


fn populate_midi_channel_combo(settings: &SettingsDialog) {
    CHANNELS.iter().for_each(|i| settings.midi_channel_combo.append_text(i));
}

fn populate_midi_combos(settings: &SettingsDialog,
                        in_name: &Option<String>, out_name: &Option<String>) {
    // populate "midi in" combo box
    settings.midi_in_combo.remove_all();
    let in_ports = MidiIn::ports().ok().unwrap_or_default();
    in_ports.iter().for_each(|i| settings.midi_in_combo.append_text(i));

    settings.midi_in_combo.set_active(None);
    let current_in_port = in_name.clone().unwrap_or_default();
    if in_ports.len() > 0 {
        let v = in_ports.iter().enumerate()
            .find(|(_, name)| &current_in_port == *name)
            .map(|(idx, _)| idx as u32)
            .or(Some(0));
        settings.midi_in_combo.set_active(v);
    };

    // populate "midi out" combo box
    settings.midi_out_combo.remove_all();
    let out_ports = MidiOut::ports().ok().unwrap_or_default();
    out_ports.iter().for_each(|i| settings.midi_out_combo.append_text(i));

    settings.midi_out_combo.set_active(None);
    let current_out_port = out_name.clone().unwrap_or_default();
    if out_ports.len() > 0 {
        let v = out_ports.iter().enumerate()
            .find(|(_, name)| &current_out_port == *name)
            .map(|(idx, _)| idx as u32)
            .or(Some(0));
        settings.midi_out_combo.set_active(v);
    };
}

fn populate_model_combo(settings: &SettingsDialog, selected: &Option<String>) {
    settings.model_combo.remove_all();

    let mut names = configs().iter().map(|c| &c.name).collect::<Vec<_>>();
    names.sort();
    for (i, &name) in names.iter().enumerate() {
        settings.model_combo.append_text(name.as_str());
    }

    let selected =
        selected.as_ref().and_then(|selected| {
            names.iter().enumerate()
                .find(|(_, &n)| *n == *selected)
                .map(|(i, _)| i as u32)
        });
    settings.model_combo.set_active(selected);
}

fn wire_autodetect_button(settings: &SettingsDialog) {
    let mut settings = settings.clone();
    settings.autodetect_button.clone().connect_clicked(move |button| {
        let mut autodetect = tokio::spawn(pod_core::midi_io::autodetect());

        let mut settings = settings.clone();
        settings.work_start(Some(button));

        glib::idle_add_local(move || {
            let cont = match autodetect.poll() {
                None => { true }
                Some(Ok((in_, out_, channel, config))) => {
                    let msg = format!("Autodetect successful!");
                    settings.work_finish("dialog-ok", &msg);

                    // update in/out port selection, channel, device
                    populate_midi_combos(&settings,
                                         &Some(in_.name.clone()), &Some(out_.name.clone()));
                    let index = midi_channel_to_combo_index(channel);
                    settings.midi_channel_combo.set_active(index);
                    populate_model_combo(&settings, &Some(config.name.clone()));
                    false
                }
                Some(Err(e)) => {
                    error!("Settings MIDI autodetect failed: {}", e);
                    let msg = format!("Autodetect failed:\n{}", e);
                    settings.work_finish("dialog-error", &msg);

                    false
                }
            };
            Continue(cont)
        });
    });
}

fn wire_test_button(settings: &SettingsDialog) {
    let settings = settings.clone();
    settings.test_button.clone().connect_clicked(move |button| {
        let midi_in = settings.midi_in_combo.active_text();
        let midi_out = settings.midi_out_combo.active_text();
        let midi_channel = settings.midi_channel_combo.active();
        let config = settings.model_combo.active_text()
            .and_then(|name| {
                configs().iter().find(|c| c.name == name)
            });

        if midi_in.is_none() || midi_out.is_none() {
            settings.set_message("dialog-warning", "Select MIDI input & output device");
            return;
        }
        if config.is_none() {
            settings.set_message("dialog-warning", "Select device type");
            return;
        }

        let midi_in = midi_in.as_ref().unwrap().to_string();
        let midi_out = midi_out.as_ref().unwrap().to_string();
        let midi_channel = midi_channel_from_combo_index(midi_channel);

        let mut test = tokio::spawn(async move {
            pod_core::midi_io::test(&midi_in, &midi_out, midi_channel, config.unwrap()).await
        });

        let mut settings = settings.clone();
        settings.work_start(Some(button));

        glib::idle_add_local(move || {
            let cont = match test.poll() {
                None => { true }
                Some(Ok((in_, out_, _))) => {
                    let msg = format!("Test successful!");
                    settings.work_finish("dialog-ok", &msg);

                    // update in/out port selection
                    // TODO: do we need to update the combo here at all?
                    populate_midi_combos(&settings,
                                         &Some(in_.name.clone()), &Some(out_.name.clone()));
                    false
                }
                Some(Err(e)) => {
                    error!("Settings MIDI test failed: {}", e);
                    let msg = format!("Test failed:\n{}", e);
                    settings.work_finish("dialog-error", &msg);

                    false
                }
            };
            Continue(cont)
        });
    });
}

pub fn wire_settings_dialog(state: Arc<Mutex<State>>, ui: &gtk::Builder) {
    let settings = SettingsDialog::new(ui);
    let settings_button: gtk::Button = ui.object("settings_button").unwrap();

    let settings_ = settings.clone();
    let state_ = state.clone();

    settings_button.connect_clicked(move |_| {
        // reset the dialog
        settings.set_interactive(true);
        settings.clear_message();

        // update in/out port selection, channel, model
        let midi_io_stop_handle = {
            let mut state = state.lock().unwrap();
            populate_midi_combos(&settings,
                                 &state.midi_in_name, &state.midi_out_name);

            let index = midi_channel_to_combo_index(state.midi_channel_num);
            settings.midi_channel_combo.set_active(index);

            let config_name = state.config.map(|c| c.name.clone());
            populate_model_combo(&settings, &config_name);

            // stop the midi thread during test
            midi_in_out_stop(&mut state)
        };

        settings.set_message("", "Waiting for MIDI...");
        settings.set_interactive(false);

        let mut midi_io_stop_wait = tokio::spawn(async {
            let results = midi_io_stop_handle.await;
            let errors = results.into_iter()
                .filter(|r| r.is_err())
                .map(|r| r.unwrap_err())
                .collect::<Vec<_>>();
            if errors.is_empty() {
                Ok(())
            } else {
                Err(anyhow!("Failed to stop {} MIDI threads", errors.len()))
            }
        });

        {
            let mut settings = settings.clone();
            glib::idle_add_local(move || {
                let cont = match midi_io_stop_wait.poll() {
                    None => { true }
                    Some(Ok(_)) => {
                        settings.work_finish("", "");
                        false
                    }
                    Some(Err(e)) => {
                        let msg = format!("Failed to stop MIDI threads:\n{}", e);
                        settings.work_finish("dialog-error", &msg);
                        false
                    }
                };
                Continue(cont)
            });
        }

        match settings.dialog.run() {
            ResponseType::Ok => {
                let midi_in = settings.midi_in_combo.active_text()
                    .and_then(|name| {
                        let name = name.as_str();
                        match MidiIn::new_for_name(name) {
                            Ok(midi) => { Some(midi) }
                            Err(err) => {
                                error!("Failed to open MIDI after settings dialog closed: {}", err);
                                None
                            }
                        }
                    });
                let midi_out = settings.midi_out_combo.active_text()
                    .and_then(|name| {
                        let name = name.as_str();
                        match MidiOut::new_for_name(name) {
                            Ok(midi) => { Some(midi) }
                            Err(err) => {
                                error!("Failed to open MIDI after settings dialog closed: {}", err);
                                None
                            }
                        }
                    });
                let config = settings.model_combo.active_text()
                    .and_then(|name| {
                        configs().iter().find(|c| c.name == name)
                    });

                let midi_channel = settings.midi_channel_combo.active();
                let midi_channel = midi_channel_from_combo_index(midi_channel);
                set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out, midi_channel, config);
            }
            _ => {
                let mut state = state.lock().unwrap();
                let names = state.midi_in_name.as_ref().and_then(|in_name| {
                    state.midi_out_name.as_ref().map(|out_name| (in_name.clone(), out_name.clone()))
                });

                // restart midi thread after test
                if let Some((in_name, out_name)) = names {
                    let midi_in = MidiIn::new_for_name(in_name.as_str())
                        .map_err(|err| {
                        error!("Unable to restart MIDI input thread for {:?}: {}", in_name, err)
                    }).ok();
                    let midi_out = MidiOut::new_for_name(out_name.as_str())
                        .map_err(|err| {
                            error!("Unable to restart MIDI output thread for {:?}: {}", out_name, err)
                        }).ok();
                    let midi_channel_num = state.midi_channel_num;
                    let quirks = state.config.map(|c| c.midi_quirks).unwrap();
                    midi_in_out_start(&mut state, midi_in, midi_out, midi_channel_num, quirks);
                }
            }
        }

        settings.dialog.hide();
    });

    populate_midi_channel_combo(&settings_);
    wire_autodetect_button(&settings_);
    wire_test_button(&settings_);
}