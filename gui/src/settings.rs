use std::sync::{Arc, Mutex};
use pod_core::pod::*;
use crate::{gtk, set_midi_in_out, State};
use pod_gtk::gtk::prelude::*;
use pod_gtk::gtk::{IconSize, ResponseType};
use crate::util::ManualPoll;

use log::*;
use pod_gtk::glib;

#[derive(Clone)]
struct SettingsDialog {
    dialog: gtk::Dialog,
    midi_in_combo: gtk::ComboBoxText,
    midi_out_combo: gtk::ComboBoxText,
    autodetect_button: gtk::Button,
    test_button: gtk::Button,
    message_label: gtk::Label,
    message_image: gtk::Image
}

impl SettingsDialog {
    fn new(ui: &gtk::Builder) -> Self {
        SettingsDialog {
            dialog: ui.object("settings_dialog").unwrap(),
            midi_in_combo: ui.object("settings_midi_in_combo").unwrap(),
            midi_out_combo: ui.object("settings_midi_out_combo").unwrap(),
            autodetect_button: ui.object("settings_autodetect_button").unwrap(),
            test_button: ui.object("settings_test_button").unwrap(),
            message_label: ui.object("settings_message_label").unwrap(),
            message_image: ui.object("settings_message_image").unwrap()
        }
    }

    fn set_interactive(&self, sensitive: bool) {
        self.dialog.set_response_sensitive(ResponseType::Ok, sensitive);
        self.midi_in_combo.set_sensitive(sensitive);
        self.midi_out_combo.set_sensitive(sensitive);
        self.autodetect_button.set_sensitive(sensitive);
        self.test_button.set_sensitive(sensitive);
    }

    fn set_message(&self, icon: &str, message: &str) {
        self.message_image.set_from_icon_name(Some(icon), IconSize::Dialog);
        self.message_label.set_label(message);
    }

    fn clear_message(&self) {
        self.message_image.set_from_icon_name(None, IconSize::Dialog);
        self.message_label.set_label(&"");
    }
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

fn wire_autodetect_button(settings: &SettingsDialog) {
    let settings = settings.clone();
    settings.autodetect_button.clone().connect_clicked(move |button| {
        let mut autodetect = tokio::spawn(pod_core::pod::autodetect());

        let spinner = gtk::Spinner::new();
        (*button).set_image(Some(&spinner));
        spinner.start();

        settings.set_interactive(false);

        let settings = settings.clone();
        glib::idle_add_local(move || {
            let cont = match autodetect.poll() {
                None => { true }
                Some(Ok((in_, out_, channel_))) => {
                    let msg = format!("Autodetect successful!");
                    settings.set_message("dialog-ok", &msg);
                    settings.set_interactive(true);
                    spinner.stop();

                    // update in/out port selection
                    populate_midi_combos(&settings,
                                         &Some(in_.name), &Some(out_.name));
                    false
                }
                Some(Err(e)) => {
                    error!("Settings MIDI autodetect failed: {}", e);
                    let msg = format!("Autodetect failed:\n{}", e);
                    settings.set_message("dialog-error", &msg);
                    settings.set_interactive(true);
                    spinner.stop();

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

        if midi_in.is_none() || midi_out.is_none() {
            settings.set_message("dialog-warning", "Select MIDI input & output device");
            return;
        }

        let midi_in = midi_in.as_ref().unwrap().to_string();
        let midi_out = midi_out.as_ref().unwrap().to_string();
        let channel = todo!();

        let mut test = tokio::spawn(async move {
            pod_core::pod::test(&midi_in, &midi_out, channel).await
        });

        let spinner = gtk::Spinner::new();
        (*button).set_image(Some(&spinner));
        spinner.start();

        settings.clear_message();
        settings.set_interactive(false);

        let settings = settings.clone();
        glib::idle_add_local(move || {
            let cont = match test.poll() {
                None => { true }
                Some(Ok((in_, out_, channel_))) => {
                    let msg = format!("Test successful!");
                    settings.set_message("dialog-ok", &msg);
                    settings.set_interactive(true);
                    spinner.stop();

                    // update in/out port selection
                    populate_midi_combos(&settings,
                                         &Some(in_.name), &Some(out_.name));
                    false
                }
                Some(Err(e)) => {
                    error!("Settings MIDI test failed: {}", e);
                    let msg = format!("Test failed:\n{}", e);
                    settings.set_message("dialog-error", &msg);
                    settings.set_interactive(true);
                    spinner.stop();

                    false
                }
            };
            Continue(cont)
        });
    });
}

pub fn wire_settings_dialog(state: Arc<Mutex<State>>, ui: &gtk::Builder) {
    let settings = SettingsDialog::new(ui);
    let settings_ = settings.clone();
    let settings_button: gtk::Button = ui.object("settings_button").unwrap();
    settings_button.connect_clicked(move |_| {
        // reset the dialog
        settings.set_interactive(true);
        settings.clear_message();

        // update in/out port selection
        {
            let state = state.lock().unwrap();
            populate_midi_combos(&settings,
                                 &state.midi_in_name, &state.midi_out_name);
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

                set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out);
            }
            _ => {}
        }

        settings.dialog.hide();
    });

    wire_autodetect_button(&settings_);
    wire_test_button(&settings_);
}