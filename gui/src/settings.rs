use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::anyhow;
use futures_util::TryFutureExt;
use pod_gtk::prelude::*;
use gtk::{IconSize, ResponseType};
use crate::{gtk, midi_in_out_start, midi_in_out_stop, set_midi_in_out, State};

use log::*;
use pod_core::config::configs;
use pod_core::midi::Channel;
use pod_core::midi_io::{MidiInPort, MidiOutPort, MidiPorts};
use pod_gtk::prelude::glib::bitflags::bitflags;
use crate::autodetect::{open, test};
use crate::usb;

#[derive(Clone)]
struct SettingsDialog {
    dialog: gtk::Dialog,
    midi_in_combo: gtk::ComboBox,
    midi_in_combo_model: gtk::ListStore,
    midi_out_combo: gtk::ComboBox,
    midi_out_combo_model: gtk::ListStore,
    midi_channel_combo: gtk::ComboBoxText,
    model_combo: gtk::ComboBoxText,
    autodetect_button: gtk::Button,
    test_button: gtk::Button,
    message_label: gtk::Label,
    message_image: gtk::Image,

    spinner: Option<gtk::Spinner>
}

bitflags! {
    pub struct EntryFlags: u8 {
        const ENTRY_HEADER = 0x01;
        const ENTRY_TEXT   = 0x02;

        const ENTRY_USB    = 0x10;
    }
}

impl SettingsDialog {
    fn new(ui: &gtk::Builder) -> Self {
        let func = |combo: &gtk::ComboBox| {
            let combo = combo.clone();

            // track "popup-shown" event for showing entries in a tree structure in the popup
            let popup_shown = Arc::new(AtomicBool::new(false));
            combo.connect_popup_shown_notify({
                let popup_shown = popup_shown.clone();
                move |combo| {
                    popup_shown.store(combo.is_popup_shown(), Ordering::Relaxed);
                }
            });

            move |layout: &gtk::CellLayout, renderer: &gtk::CellRenderer, model: &gtk::TreeModel, iter: &gtk::TreeIter| {
                let popup_shown = popup_shown.load(Ordering::Relaxed);

                let entry_type = EntryFlags::from_bits(
                    model.value(iter, 1).get::<u8>().unwrap()
                ).unwrap();
                if entry_type.contains(EntryFlags::ENTRY_HEADER) {
                    // header text
                    renderer.set_sensitive(false);
                    renderer.set_visible(popup_shown);
                    renderer.set_padding(0, 0);
                    renderer.set_properties(&[
                        ("weight", &700)
                    ]);
                } else {
                    // normal entry or text entry
                    renderer.set_sensitive(!entry_type.contains(EntryFlags::ENTRY_TEXT));
                    renderer.set_visible(true);
                    let padding = if popup_shown { 10 } else { 0 };
                    renderer.set_padding(padding, 0);
                    renderer.set_properties(&[
                        ("weight", &400)
                    ]);
                }
            }
        };

        let midi_in_combo = ui.object::<gtk::ComboBox>("settings_midi_in_combo").unwrap();

        let renderer = gtk::CellRendererText::new();
        midi_in_combo.clear();
        midi_in_combo.pack_start(&renderer, true);
        midi_in_combo.add_attribute(&renderer, "text", 0);
        midi_in_combo.set_cell_data_func(&renderer, Some(Box::new(func(&midi_in_combo))));

        let midi_in_combo_model = gtk::ListStore::new(&[glib::Type::STRING, glib::Type::U8]);
        midi_in_combo.set_model(Some(&midi_in_combo_model));

        let midi_out_combo = ui.object::<gtk::ComboBox>("settings_midi_out_combo").unwrap();

        let renderer = gtk::CellRendererText::new();
        midi_out_combo.clear();
        midi_out_combo.pack_start(&renderer, true);
        midi_out_combo.add_attribute(&renderer, "text", 0);
        midi_out_combo.set_cell_data_func(&renderer, Some(Box::new(func(&midi_out_combo))));

        let midi_out_combo_model = gtk::ListStore::new(&[glib::Type::STRING, glib::Type::U8]);
        midi_out_combo.set_model(Some(&midi_out_combo_model));

        // attach combo
        let combos = vec![
            (midi_in_combo.clone(), midi_out_combo.clone()),
            (midi_out_combo.clone(), midi_in_combo.clone()),
        ];
        for (src, target) in combos {
            src.connect_active_notify(move |combo| {
                let Some((name, flags)) = combo_get_active(combo) else { return };
                if !flags.contains(EntryFlags::ENTRY_USB) { return };

                let model = target.model().unwrap();
                let store = model.dynamic_cast_ref::<gtk::ListStore>().unwrap();
                let item = combo_model_find(store, &Some(name));
                target.set_active(item);
            });

        }

        SettingsDialog {
            dialog: ui.object("settings_dialog").unwrap(),
            midi_in_combo,
            midi_in_combo_model,
            midi_out_combo,
            midi_out_combo_model,
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
        self.message_label.set_markup(message);
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

fn combo_model_populate(model: &gtk::ListStore, midi_devices: &Vec<String>, usb_devices: &Vec<String>) {
    model.clear();

    let mut n: u32 = 0;
    let mut next = || { let r = n; n += 1; Some(n) };

    let mut add = |entry: &str, flags: EntryFlags| {
        let data: [(u32, &dyn ToValue); 2] = [ (0, &entry), (1, &flags.bits()) ];
        model.insert_with_values(next(), &data);
    };

    add("MIDI", EntryFlags::ENTRY_HEADER);
    if !midi_devices.is_empty() {
        for name in midi_devices {
            add(name, EntryFlags::empty());
        }
    } else {
        add("No devices found...", EntryFlags::ENTRY_TEXT);
    }

    add("USB", EntryFlags::ENTRY_HEADER);
    if !usb_devices.is_empty() {
        for name in usb_devices {
            add(name, EntryFlags::ENTRY_USB);
        }
    } else {
        add("No devices found...", EntryFlags::ENTRY_TEXT);
    }
}

fn combo_model_find(model: &gtk::ListStore, value: &Option<String>) -> Option<u32> {
    let iter = model.iter_first();
    if let Some(iter) = iter {
        let mut has_value = true;
        let mut n = 0;
        while has_value {
            let flags = EntryFlags::from_bits(
                model.value(&iter, 1).get::<u8>().unwrap()
            ).unwrap();
            if (flags & (EntryFlags::ENTRY_HEADER | EntryFlags::ENTRY_TEXT)) != EntryFlags::empty() {
                n += 1;
                has_value = model.iter_next(&iter);
                continue;
            }
            let name = model.value(&iter, 0).get::<String>().unwrap();
            if value.is_none() || value.as_ref().map(|v| *v == name).unwrap_or_default() {
                return Some(n);
            }

            n += 1;
            has_value = model.iter_next(&iter);
        }
    }

    None
}

fn combo_get_active(combo: &gtk::ComboBox) -> Option<(String, EntryFlags)> {
    let Some(model) = combo.model() else {
        return None;
    };
    let Some(iter) = combo.active_iter() else {
        return None;
    };

    let model = model.dynamic_cast_ref::<gtk::ListStore>().unwrap();
    let name = model.value(&iter, 0).get::<String>().unwrap();
    let flags = EntryFlags::from_bits(
        model.value(&iter, 1).get::<u8>().unwrap()
    ).unwrap();
    Some((name, flags))
}

fn populate_midi_combos(settings: &SettingsDialog,
                        in_name: &Option<String>, out_name: &Option<String>) {
    // populate "midi in" combo box
    let midi_ports = MidiInPort::ports().ok().unwrap_or_default();
    let usb_ports = usb::usb_list_devices();
    combo_model_populate(&settings.midi_in_combo_model, &midi_ports, &usb_ports);
    let active = combo_model_find(&settings.midi_in_combo_model, in_name);
    settings.midi_in_combo.set_active(active);

    // populate "midi out" combo box
    let midi_ports = MidiOutPort::ports().ok().unwrap_or_default();
    combo_model_populate(&settings.midi_out_combo_model, &midi_ports, &usb_ports);
    let active = combo_model_find(&settings.midi_out_combo_model, out_name);
    settings.midi_out_combo.set_active(active);
}

fn populate_model_combo(settings: &SettingsDialog, selected: &Option<String>) {
    settings.model_combo.remove_all();

    let mut names = configs().iter().map(|c| &c.name).collect::<Vec<_>>();
    names.sort();
    for &name in names.iter() {
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
    let settings = settings.clone();
    settings.autodetect_button.clone().connect_clicked(move |button| {
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        tokio::spawn(async move {
            let res = pod_core::midi_io::autodetect(None).await;
            tx.send(res).ok();
        });

        let mut settings = settings.clone();
        settings.work_start(Some(button));

        rx.attach(None, move |autodetect| {
            match autodetect {
                Ok((in_, out_, channel, config)) => {
                    let msg = format!("Autodetect successful!");
                    settings.work_finish("dialog-ok", &msg);

                    // update in/out port selection, channel, device
                    populate_midi_combos(&settings,
                                         &Some(in_.name()), &Some(out_.name()));
                    let index = midi_channel_to_combo_index(channel);
                    settings.midi_channel_combo.set_active(index);
                    populate_model_combo(&settings, &Some(config.name.clone()));
                }
                Err(e) => {
                    error!("Settings MIDI autodetect failed: {}", e);
                    let msg = format!("Autodetect failed:\n\n{}", e);
                    settings.work_finish("dialog-error", &msg);
                }
            };
            Continue(false)
        });
    });
}

fn wire_test_button(settings: &SettingsDialog) {
    let settings = settings.clone();
    settings.test_button.clone().connect_clicked(move |button| {
        let midi_in = combo_get_active(&settings.midi_in_combo);
        let midi_in_is_usb = midi_in.as_ref().map(|(_,flags)| flags.contains(EntryFlags::ENTRY_USB)).unwrap_or(false);
        let midi_out = combo_get_active(&settings.midi_out_combo);
        let midi_out_is_usb = midi_out.as_ref().map(|(_,flags)| flags.contains(EntryFlags::ENTRY_USB)).unwrap_or(false);
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

        let is_usb = midi_in_is_usb || midi_out_is_usb;
        let midi_in = midi_in.map(|(n,_)| n).unwrap();
        let midi_out = midi_out.map(|(n,_)| n).unwrap();
        let midi_channel = midi_channel_from_combo_index(midi_channel);


        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        tokio::spawn(async move {
            let res = test(&midi_in, &midi_out, midi_channel, is_usb, config.unwrap()).await;
            tx.send(res).ok();
        });

        let mut settings = settings.clone();
        settings.work_start(Some(button));

        rx.attach(None, move |test| {
            match test {
                Ok((in_, out_, _)) => {
                    let msg = format!("Test successful!");
                    settings.work_finish("dialog-ok", &msg);

                    // update in/out port selection
                    // TODO: do we need to update the combo here at all?
                    populate_midi_combos(&settings,
                                         &Some(in_.name()), &Some(out_.name()));
                }
                Err(e) => {
                    error!("Settings MIDI test failed: {}", e);
                    let msg = format!("Test failed:\n\n{}", e);
                    settings.work_finish("dialog-error", &msg);
                }
            };
            Continue(false)
        });
    });
}

pub fn create_settings_action(state: Arc<Mutex<State>>, ui: &gtk::Builder) -> gio::ActionEntry<gtk::Application> {
    let settings = SettingsDialog::new(ui);

    populate_midi_channel_combo(&settings);
    wire_autodetect_button(&settings);
    wire_test_button(&settings);

    gio::ActionEntry::builder("preferences").activate(move |app: &gtk::Application, _, _| {
        settings.dialog.set_application(Some(app));
        let window = app.windows().iter()
            .find(|w| w.downcast_ref::<gtk::ApplicationWindow>().is_some())
            .cloned();
        if let Some(window) = window {
            settings.dialog.set_transient_for(Some(&window));
        }

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

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        tokio::spawn(async move {
            let results = midi_io_stop_handle.await;
            let errors = results.into_iter()
                .filter(|r| r.is_err())
                .map(|r| r.unwrap_err())
                .collect::<Vec<_>>();
            let res = if errors.is_empty() {
                Ok(())
            } else {
                Err(anyhow!("Failed to stop {} MIDI threads", errors.len()))
            };
            tx.send(res).ok();
        });

        {
            let mut settings = settings.clone();
            rx.attach(None, move |midi_io_stop_wait| {
                match midi_io_stop_wait {
                    Ok(_) => {
                        settings.work_finish("", "");
                    }
                    Err(e) => {
                        let msg = format!("Failed to stop MIDI threads:\n{}", e);
                        settings.work_finish("dialog-error", &msg);
                    }
                };
                Continue(false)
            });
        }

        match settings.dialog.run() {
            ResponseType::Ok => {
                let midi_in = combo_get_active(&settings.midi_in_combo);
                let midi_in_is_usb = midi_in.as_ref().map(|(_,flags)| flags.contains(EntryFlags::ENTRY_USB)).unwrap_or(false);
                let midi_out = combo_get_active(&settings.midi_out_combo);
                let midi_out_is_usb = midi_out.as_ref().map(|(_,flags)| flags.contains(EntryFlags::ENTRY_USB)).unwrap_or(false);
                let midi_channel = settings.midi_channel_combo.active();
                let config = settings.model_combo.active_text()
                    .and_then(|name| {
                        configs().iter().find(|c| c.name == name)
                    });

                let is_usb = midi_in_is_usb || midi_out_is_usb;
                let midi_in = midi_in.map(|(n,_)| n);
                let midi_out = midi_out.map(|(n,_)| n);
                let midi_channel = midi_channel_from_combo_index(midi_channel);

                let (midi_in, midi_out) = midi_in.zip(midi_out)
                    .and_then(|(midi_in, midi_out)| {
                        match open(&midi_in, &midi_out, is_usb) {
                            Ok(v) => { Some(v) },
                            Err(err) => {
                                error!("Failed to open MIDI after settings dialog closed: {}", err);
                                None
                            }
                        }
                    })
                    .unzip();
                set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out, midi_channel, config);
            }
            _ => {
                let mut state = state.lock().unwrap();
                let names = state.midi_in_name.as_ref().zip(state.midi_out_name.as_ref());
                let is_usb = state.midi_is_usb;
                let (midi_in, midi_out) = names
                    .and_then(|(midi_in, midi_out)| {
                        match open(&midi_in, &midi_out, is_usb) {
                            Ok(v) => { Some(v) },
                            Err(err) => {
                                error!("Failed to restart MIDI after settings dialog canceled: {}", err);
                                None
                            }
                        }
                    }).unzip();
                let midi_channel_num = state.midi_channel_num;
                let quirks = state.config.map(|c| c.midi_quirks).unwrap();
                midi_in_out_start(&mut state, midi_in, midi_out, midi_channel_num,
                                  quirks, false);
            }
        }

        settings.dialog.hide();

    }).build()
}