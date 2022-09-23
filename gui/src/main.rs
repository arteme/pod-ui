mod opts;
mod util;
mod settings;
mod panic;
mod registry;
mod empty;
mod widgets;

use anyhow::*;

use pod_gtk::*;
use pod_gtk::gtk::prelude::*;
use pod_core::pod::*;
use pod_core::controller::Controller;
use pod_core::program;
use log::*;
use std::sync::{Arc, atomic, Mutex, RwLock};
use pod_core::model::{AbstractControl, Button, Config, Control, DeviceFlags, VirtualSelect};
use pod_core::config::{config_for_id, configs, GUI, MIDI, UNSET};
use crate::opts::*;
use pod_core::midi::{Channel, MidiMessage};
use tokio::sync::{broadcast, mpsc, oneshot};
use std::thread;
use pod_core::store::{Event, Signal, Store};
use core::result::Result::Ok;
use std::collections::HashMap;
use std::option::IntoIter;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use arc_swap::{ArcSwap, ArcSwapOption};
use clap::{Args, Command, FromArgMatches};
use dyn_iter::DynIter;
use maplit::*;
use crate::settings::*;
use result::prelude::*;
use pod_core::dump::ProgramsDump;
use pod_core::edit::EditBuffer;
use pod_gtk::gtk::gdk;
use crate::panic::wire_panic_indicator;
use crate::registry::{init_module, init_module_controls, InitializedInterface, register_module};
use crate::util::ToSome;
use crate::widgets::*;


const MIDI_OUT_CHANNEL_CAPACITY: usize = 512;
const UI_EVENT_CHANNEL_CAPACITY: usize = 128;

#[derive(Clone, Debug)]
pub enum UIEvent {
    NewMidiConnection,
    NewEditBuffer,
    NewDevice,
    MidiTx,
    MidiRx,
    Modified(usize, bool),
    Panic,
    Quit
}

pub struct DetectedDevVersion {
    name: String,
    ver: String
}

pub struct State {
    pub midi_in_name: Option<String>,
    pub midi_in_cancel: Option<oneshot::Sender<()>>,

    pub midi_out_name: Option<String>,
    pub midi_out_cancel: Option<oneshot::Sender<()>>,

    pub midi_in_tx: mpsc::UnboundedSender<MidiMessage>,
    pub midi_out_tx: broadcast::Sender<MidiMessage>,
    pub ui_event_tx: broadcast::Sender<UIEvent>,

    pub midi_channel_num: u8,
    pub config: Arc<RwLock<&'static Config>>,
    pub interface: InitializedInterface,
    pub edit_buffer: Arc<ArcSwap<Mutex<EditBuffer>>>,
    pub dump: Arc<ArcSwap<Mutex<ProgramsDump>>>,
    pub detected: Arc<ArcSwapOption<DetectedDevVersion>>
}

static UI_CONTROLS: Lazy<HashMap<String, Control>> = Lazy::new(|| {
    convert_args!(hashmap!(
        "program" => VirtualSelect::default(),
        "program:prev" => VirtualSelect::default(),
        "program_num" => VirtualSelect::default(),
        "load_button" => Button::default(),
        "load_patch_button" => Button::default(),
        "load_all_button" => Button::default(),
        "store_button" => Button::default(),
        "store_patch_button" => Button::default(),
        "store_all_button" => Button::default(),

        /// Set if device config contains DeviceFlags::MANUAL_MODE
        "manual_mode_present" => VirtualSelect::default(),
    ))
});


pub fn set_midi_in_out(state: &mut State, midi_in: Option<MidiIn>, midi_out: Option<MidiOut>,
                       midi_channel: u8, config: Option<&'static Config>) -> bool {
    state.midi_in_cancel.take().map(|cancel| cancel.send(()));
    state.midi_out_cancel.take().map(|cancel| cancel.send(()));

    let config_changed = match (config, *state.config.read().unwrap()) {
        (Some(a), b) => { *a != *b }
        _ => { false }
    };
    if config_changed {
        // config changed, update config & edit buffer
        let config = config.unwrap();
        {
            let mut c = state.config.write().unwrap();
            *c = config;
        }

        state.interface = init_module(config).unwrap();

        state.edit_buffer.store(state.interface.edit_buffer.clone());
        state.dump.store(state.interface.dump.clone());

        info!("Installing config {:?}", &config.name);

        state.ui_event_tx.send(UIEvent::NewEditBuffer);
    }

    if midi_in.is_none() || midi_out.is_none() {
        warn!("Not starting MIDI because in/out is None");
        state.midi_in_name = None;
        state.midi_in_cancel = None;
        state.midi_out_name = None;
        state.midi_out_cancel = None;
        state.ui_event_tx.send(UIEvent::NewMidiConnection);
        state.midi_channel_num = 0;
        return config_changed;
    }

    let mut midi_in = midi_in.unwrap();
    let mut midi_out = midi_out.unwrap();

    let (in_cancel_tx, mut in_cancel_rx) = tokio::sync::oneshot::channel::<()>();
    let (out_cancel_tx, mut out_cancel_rx) = tokio::sync::oneshot::channel::<()>();

    state.midi_in_name = Some(midi_in.name.clone());
    state.midi_in_cancel = Some(in_cancel_tx);

    state.midi_out_name = Some(midi_out.name.clone());
    state.midi_out_cancel = Some(out_cancel_tx);

    state.midi_channel_num = midi_channel;
    state.ui_event_tx.send(UIEvent::NewMidiConnection);

    // midi in
    {
        let midi_in_tx = state.midi_in_tx.clone();
        let ui_event_tx = state.ui_event_tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(bytes) = midi_in.recv() => {
                        match MidiMessage::from_bytes(bytes) {
                            Ok(msg) => { midi_in_tx.send(msg); () },
                            Err(err) => error!("Error deserializing MIDI message: {}", err)
                        }
                        ui_event_tx.send(UIEvent::MidiRx);
                    }
                    _ = &mut in_cancel_rx => {
                        return;
                    }
                }
            }
        });
    }

    // midi out
    {
        let mut midi_out_rx = state.midi_out_tx.subscribe();
        let ui_event_tx = state.ui_event_tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = midi_out_rx.recv() => {
                        match msg {
                            Ok(msg) => {
                                let bytes = msg.to_bytes();
                                midi_out.send(&bytes);
                                ui_event_tx.send(UIEvent::MidiTx);
                            }
                            Err(err) => {
                                error!("MIDI OUT thread rx error: {:?}", err);
                            }
                        }
                    }
                    _ = &mut out_cancel_rx => {
                        return;
                    }
                }
            }
        });
    }

    // we assume that something changed -- either the config or the midi settings
    // so signal a new device ping!
    state.ui_event_tx.send(UIEvent::NewDevice);

    config_changed
}


fn program_change(dump: &mut ProgramsDump, edit_buffer: &mut EditBuffer,
                  ui_controller: &mut Controller, program: u8, origin: u8) -> bool {
    let program_range = 1 ..= dump.program_num();
    let prev_program = ui_controller.get("program:prev").unwrap_or(0) as usize;
    if program_range.contains(&prev_program) {
        // store edit buffer to the programs dump
        let prev_page = prev_program - 1;
        let program_data = program::store_patch_dump_ctrl(edit_buffer);
        program::load_patch_dump(dump, prev_page, program_data.as_slice(), origin);
        dump.set_modified(prev_page, edit_buffer.modified());
    }

    // program == config.program_num is converted to 999, which is s special id for "tuner"
    let program_num = ui_controller.get("program_num").unwrap() as u8;
    let program = if program == (program_num + 1) { 999 } else { program as usize };
    let mut modified = false;
    if program_range.contains(&program) {
        // load program dump into the edit buffer
        let page = program - 1;
        let data = dump.data(page).unwrap();

        // In case of program change, always send a signal that the data change is coming
        // from MIDI so that the GUI gets updated, but the MIDI does not
        program::load_patch_dump_ctrl(edit_buffer, data, MIDI);
        modified = dump.modified(page);
        edit_buffer.set_modified(modified);
    }

    ui_controller.set("program", program as u16, origin);
    ui_controller.set("program:prev", program as u16, origin);

    modified
}

enum Program {
    EditBuffer,
    Current,
    Number(usize),
    All
}

fn program_dump_message(
    program: Program, dump: &mut ProgramsDump, edit_buffer: &mut EditBuffer,
    ui_controller: &Controller, ui_event_tx: &broadcast::Sender<UIEvent>,
    config: &Config
) -> Option<Vec<MidiMessage>> {
    let cur_program = ui_controller.get(&"program").unwrap() as usize;
    let cur_program_valid = cur_program > 0 && cur_program < dump.program_num();
    let save_buffer = match program {
        Program::EditBuffer => false,
        Program::Number(n) if n != cur_program => false,
        _ => true
    };
    if edit_buffer.modified() && save_buffer && cur_program_valid {
        let buffer = dump.data_mut(cur_program - 1).unwrap();
        program::store_patch_dump_ctrl_buf(&edit_buffer, buffer);
        edit_buffer.set_modified(false);
    }

    match program {
        Program::EditBuffer => {
            vec![MidiMessage::ProgramEditBufferDump {
                ver: 0,
                data: program::store_patch_dump_ctrl(&edit_buffer)
            }].some()
        }
        Program::Current if !cur_program_valid => { None }
        Program::Current => {
            let patch = cur_program - 1;
            dump.set_modified(patch, false);
            ui_event_tx.send(UIEvent::Modified(patch, false));

            vec![MidiMessage::ProgramPatchDump {
                patch: patch as u8,
                ver: 0,
                data: program::store_patch_dump(&dump, patch)
            }].some()
        }
        Program::Number(n) => {
            let patch = n - 1;
            dump.set_modified(patch, false);
            ui_event_tx.send(UIEvent::Modified(patch, false));

            vec![MidiMessage::ProgramPatchDump {
                patch: patch as u8,
                ver: 0,
                data: program::store_patch_dump(&dump, patch)
            }].some()
        }
        Program::All => {
            dump.set_all_modified(false);
            for i in 0 .. dump.program_num() {
                ui_event_tx.send(UIEvent::Modified(i, false));
            }

            if config.flags.contains(DeviceFlags::ALL_PROGRAMS_DUMP) {
                // all programs in a single dump message
                vec![MidiMessage::AllProgramsDump {
                    ver: 0,
                    data: program::store_all_dump(&dump)
                }].some()
            } else {
                // individual program dump messages for each program
                (0 .. config.program_num)
                    .map(|patch| {
                        MidiMessage::ProgramPatchDump {
                            patch: patch as u8,
                            ver: 0,
                            data: program::store_patch_dump(&dump, patch)
                        }
                    })
                    .collect::<Vec<_>>()
                    .some()
            }
        }
    }

}


fn set_current_program_modified(edit: &mut EditBuffer,
                                ui_controller: &Controller,
                                ui_event_tx: &broadcast::Sender<UIEvent>) {
    let cur_program = ui_controller.get(&"program").unwrap() as usize;
    let program_num = ui_controller.get(&"program_num").unwrap() as usize;
    let cur_program_valid = cur_program > 0 && cur_program < program_num;
    if cur_program_valid {
        let current_page = cur_program - 1;
        ui_event_tx.send(UIEvent::Modified(current_page, true));
    }
    edit.set_modified(true);
}

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

/// Called when a new edit buffer & UI have been connected
fn new_device_ping(state: &State) {
    // Request device id from the POD device
    state.midi_out_tx
        .send(MidiMessage::UniversalDeviceInquiry { channel: state.midi_channel_num }).unwrap();
    // Request all programs dump from the POD device
    state.midi_out_tx.send(MidiMessage::AllProgramsDumpRequest).unwrap();
}

fn update_ui_from_state(state: &State, ui_controller: &mut Controller) {
    let config = state.config.read().unwrap();

    // Update "program_num" value in the ui_controller
    ui_controller.set_full("program_num",
                           config.program_num as u16,
                           UNSET, Signal::Force);

    // Update the "manual" flag in the ui_controller
    ui_controller.set_full("manual_mode_present",
                           config.flags.contains(DeviceFlags::MANUAL_MODE) as u16,
                           UNSET, Signal::Force);
}

#[tokio::main]
async fn main() -> Result<()> {
    let version = env!("GIT_VERSION");
    let _guard = sentry::init((option_env!("SENTRY_DSN"), sentry::ClientOptions {
        release: Some(version.into()),
        ..Default::default()
    }));
    let sentry_enabled = _guard.is_enabled();
    simple_logger::init()?;

    register_module(pod_mod_pod2::module());
    register_module(pod_mod_pocket::module());

    let help_text = generate_help_text()?;
    let cli = Command::new("Pod UI")
        .version(version)
        .after_help(&*help_text)
        .after_long_help(&*help_text);

    let cli = Opts::augment_args(cli);
    let opts: Opts = Opts::from_arg_matches(&cli.get_matches())?;
    drop(help_text);

    let (midi_in_tx, mut midi_in_rx) = mpsc::unbounded_channel::<MidiMessage>();
    let (midi_out_tx, midi_out_rx) = broadcast::channel::<MidiMessage>(MIDI_OUT_CHANNEL_CAPACITY);
    let (ui_event_tx, mut ui_event_rx) = broadcast::channel(UI_EVENT_CHANNEL_CAPACITY);

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let state = {
        // From the start, chose the first registered config (POD 2.0)
        // and initialize it's interface. Later auto-detection may override
        // this and initialize a different interface to replace this one...

        // TODO: how can we defer the config/interface setting so that we don't need
        //       to provide bogus (empty) configs and don't need to dissect interface
        //       by hand?
        let config = configs().get(0).unwrap();
        let interface = init_module(config)?;
        let edit_buffer = interface.edit_buffer.clone();
        let dump = interface.dump.clone();

        Arc::new(Mutex::new(State {
            midi_in_name: None,
            midi_in_cancel: None,
            midi_out_name: None,
            midi_out_cancel: None,
            midi_in_tx,
            midi_out_tx,
            ui_event_tx: ui_event_tx.clone(),
            midi_channel_num: 0,
            config: Arc::new(RwLock::new(config)),
            interface,
            edit_buffer: Arc::new(ArcSwap::from(edit_buffer)),
            dump: Arc::new(ArcSwap::from(dump)),
            detected: Arc::new(ArcSwapOption::empty())
        }))
    };
    let (edit_buffer, dump, config) = {
        let state = state.lock().unwrap();
        (state.edit_buffer.clone(), state.dump.clone(), state.config.clone())
    };

    // autodetect/open midi
    let autodetect = match (&opts.input, &opts.output) {
        (None, None) => true,
        _ => false
    };
    let (midi_in, midi_out, midi_channel, detected_config) =
        if autodetect {
            match pod_core::pod::autodetect().await {
                Ok((midi_in, midi_out, channel, config)) => {
                    (Some(midi_in), Some(midi_out), channel, Some(config))
                }
                Err(err) => {
                    error!("MIDI autodetect failed: {}", err);
                    (None, None, 0, None)
                }
            }
        } else {
            let midi_in =
                opts.input.map(MidiIn::new_for_address).invert()?;
            let midi_out =
                opts.output.map(MidiOut::new_for_address).invert()?;
            let midi_channel = opts.channel.unwrap_or(0);
            let config =
                opts.model.map(|str| config_for_str(&str)).invert()?;

            (midi_in, midi_out, midi_channel, config)
        };

    // moving this to below the panic handler so that early crashed
    // in the midi thread are shown in the UI
    //set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out, midi_channel);

    // midi channel number
    let midi_channel_num = Arc::new(AtomicU8::new(0));

    let mut ui_callbacks = Callbacks::new();

    let ui = gtk::Builder::from_string(include_str!("ui.glade"));
    let ui_objects = ObjectList::new(&ui);
    let ui_controller = Arc::new(Mutex::new(Controller::new((*UI_CONTROLS).clone())));
    wire(ui_controller.clone(), &ui_objects, &mut ui_callbacks)?;

    let program_grid = ArcSwap::from(Arc::new(ProgramGrid::new(32)));

    let title = format!("POD UI {}", version);

    let window: gtk::Window = ui.object("ui_win").unwrap();
    window.set_title(&title);
    window.connect_delete_event(move |_, _| {
        info!("Quitting");
        ui_event_tx.send(UIEvent::Quit);
        Inhibit(true)
    });
    let transfer_icon_up: gtk::Label = ui.object("transfer_icon_up").unwrap();
    let transfer_icon_down: gtk::Label = ui.object("transfer_icon_down").unwrap();
    transfer_icon_up.set_opacity(0.0);
    transfer_icon_down.set_opacity(0.0);
    let header_bar: gtk::HeaderBar = ui.object("header_bar").unwrap();

    wire_settings_dialog(state.clone(), &ui);
    wire_panic_indicator(state.clone());
    // wire open button
    let open_button = ui.object::<gtk::ToggleButton>("open_button").unwrap();
    open_button.connect_clicked({
        let window = window.clone();
        let grid = ui.object::<gtk::Grid>("program_grid").unwrap();
        move |button| {
            let is_active = button.is_active();
            // dynamically look up the current ProgramGrid widget from the UI
            ObjectList::from_widget(&grid)
                .objects_by_type::<ProgramGrid>().next()
                .map(|g| {
                    grid.remove(g);
                    let w = if is_active { g.num_pages() * 2 } else { 2 };
                    grid.attach(g, 0, 1, w as i32, 18);
                    g.set_open(is_active);
                });
            if !button.is_active() {
                window.resize(1,1);
            }
        }
    });

    let config_changed =
        set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out, midi_channel, detected_config);
    if !config_changed {
        // No new edit buffer / interface was initialized above,
        // but make sure the initial interface gets connected to the UI
        let state = state.lock().unwrap();
        state.ui_event_tx.send(UIEvent::NewEditBuffer)?;
        state.ui_event_tx.send(UIEvent::NewDevice)?;
    }

    let css = gtk::CssProvider::new();
    css.load_from_data(include_str!("default.css").as_bytes())
        .unwrap_or_else(|e| error!("Failed to load default CSS: {}", e.message()));
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Error initializing GTK CSS provider"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    let device_box: gtk::Box = ui.object("device_box").unwrap();
    // TODO: WHAT TO DO HERE?

    // midi ----------------------------------------------------

    // controller / ui controller / midi reply -> midi out
    {
        let config = config.clone();
        let edit_buffer = edit_buffer.clone();
        let dump = dump.clone();
        let ui_controller = ui_controller.clone();
        let midi_out_tx = state.lock().unwrap().midi_out_tx.clone();
        let ui_event_tx = state.lock().unwrap().ui_event_tx.clone();
        let mut ui_event_rx = state.lock().unwrap().ui_event_tx.subscribe();
        let midi_channel_num = midi_channel_num.clone();
        tokio::spawn(async move {
            let make_cc = |name: &str, controller: &Controller| -> Option<Vec<MidiMessage>> {
                let control = controller.get_config(name);
                if control.is_none() {
                    warn!("Control {:?} not found!", name);
                    return None;
                }

                let (value, origin) = controller.get_origin(name).unwrap();
                if origin != GUI {
                    // not forwarding MIDI events back to MIDI,
                    // value_to_midi() below may overflow on union control getting incorrect data
                    return None;
                }
                let control = control.unwrap();
                let cc = control.get_cc();
                if cc.is_none() {
                    return None; // skip virtual controls
                }

                // Map the control address to a list of controls to make CC messages for.
                // Typically this will be a single-element list with the same control
                // that was resolved by name. For multibyte controls this will be a list
                // of [tail control, head control], sorted this way specifically because
                // we want to sent the lower bytes first.
                let controls: Vec<&Control> = control.get_addr()
                    .filter(|(_, size)| *size > 1)
                    .map(|(addr, size)| {
                        // multibyte control
                        let config = config.read().unwrap();
                        config.addr_to_control_vec((addr + size -1) as usize, true)
                            .into_iter().map(|(_, c)| c).collect()
                    })
                    // single byte control, or control without address
                    .unwrap_or_else(|| vec![control]);

                let channel = midi_channel_num.load(Ordering::Relaxed);
                let channel = if channel == Channel::all() { 0 } else { channel };

                let messages = controls.into_iter().map(|control| {
                    let value = control.value_to_midi(value);
                    let cc = control.get_cc();
                    MidiMessage::ControlChange { channel, control: cc.unwrap(), value }
                }).collect();
                Some(messages)
            };
            let make_pc = || {
                let channel = midi_channel_num.load(Ordering::Relaxed);
                let channel = if channel == Channel::all() { 0 } else { channel };
                let ui_controller = ui_controller.lock().unwrap();
                let mut v = ui_controller.get(&"program").unwrap();
                // program = 999 is a special case for "tuner", which is "config.program_num + 1"
                if v == 999 {
                    let program_num = ui_controller.get(&"program_num").unwrap();
                    v = program_num + 1;
                }
                Some(vec![ MidiMessage::ProgramChange { channel, program: v as u8 } ])
            };
            let make_dump_request = |program: Program| {
                let ui_controller = ui_controller.lock().unwrap();
                let req = match program {
                    Program::EditBuffer => Some(MidiMessage::ProgramEditBufferDumpRequest),
                    Program::Current => {
                        let current = ui_controller.get(&"program").unwrap();
                        if current > 0 {
                            Some(MidiMessage::ProgramPatchDumpRequest { patch: (current - 1) as u8 })
                        } else {
                            None
                        }
                    }
                    Program::All => Some(MidiMessage::AllProgramsDumpRequest),
                    _ => None // we never do number request (yet!)
                };
                req.map(|req| vec![ req ])
            };
            let make_dump = |program: Program| {
                let edit_buffer = edit_buffer.load();
                let dump = dump.load();
                let mut edit_buffer = edit_buffer.lock().unwrap();
                let mut dump = dump.lock().unwrap();
                let ui_controller = ui_controller.lock().unwrap();
                let config = config.read().unwrap();

                program_dump_message(program, &mut dump, &mut edit_buffer,
                                     &ui_controller, &ui_event_tx, &config)
            };

            let mut rx = None;
            let mut ui_rx = ui_controller.lock().unwrap().subscribe();

            loop {
                if rx.is_none() {
                    let edit_buffer = edit_buffer.load();
                    let edit_buffer = edit_buffer.lock().unwrap();
                    rx = Some(edit_buffer.subscribe());
                }

                let mut messages: Option<Vec<MidiMessage>> = None;
                let mut origin: u8 = UNSET;
                tokio::select! {
                    controller_event = rx.as_mut().unwrap().recv() => {
                          match controller_event {
                              Ok(Event { key: name, origin: o, .. }) => {
                                  messages = make_cc(&name, &edit_buffer.load().lock().unwrap().controller_locked());
                                  origin = o;
                              }
                              _ => {}
                          }
                    }
                    ui_controller_event = ui_rx.recv() => {
                          match ui_controller_event {
                              Ok(Event { key, origin: o, .. }) => {
                                  messages = match key.as_str() {
                                      "program" => make_pc(),
                                      "load_button" => make_dump_request(Program::EditBuffer),
                                      "load_patch_button" => make_dump_request(Program::Current),
                                      "load_all_button" => make_dump_request(Program::All),
                                      "store_button" => make_dump(Program::EditBuffer),
                                      "store_patch_button" => make_dump(Program::Current),
                                      "store_all_button" => make_dump(Program::All),
                                      _ => None
                                  };
                                  origin = o;
                              }
                              _ => {}
                          }
                    }
                    ui_event = ui_event_rx.recv() => {
                          match ui_event {
                              Ok(UIEvent::NewEditBuffer) => rx = None,
                              _ => {}
                          }
                      }
                }
                if rx.is_none() || origin == MIDI || messages.is_none() {
                    continue;
                }
                let config = (&*config.read().unwrap()).clone();
                let mut messages: DynIter<MidiMessage> = DynIter::new(messages.unwrap_or_default().into_iter());
                while let Some(message) = messages.next() {
                    let send_buffer = match message {
                        MidiMessage::ControlChange { .. } => {
                            // CC from GUI layer -> set modified flag
                            set_current_program_modified(
                                &mut edit_buffer.load().lock().unwrap(),
                                &ui_controller.lock().unwrap(),
                                &ui_event_tx
                            );
                            false
                        }
                        MidiMessage::ProgramChange { program, .. } => {
                            let edit_buffer = edit_buffer.load();
                            let dump = dump.load();
                            let mut edit_buffer = edit_buffer.lock().unwrap();
                            let mut dump = dump.lock().unwrap();
                            let mut ui_controller = ui_controller.lock().unwrap();
                            program_change(&mut dump, &mut edit_buffer, &mut ui_controller, program, GUI)
                        }
                        _ => { false }
                    };

                    if send_buffer {
                        // chain up an "edit buffer dump" message
                        messages = DynIter::new(
                            make_dump(Program::EditBuffer).unwrap().into_iter()
                                .chain(messages)
                        );

                        if !config.flags.contains(DeviceFlags::MODIFIED_BUFFER_PC_AND_EDIT_BUFFER) {
                            // skip sending PC here
                            continue;
                        }
                    }

                    // check if we need to chain up an "edit buffer dump request
                    match message {
                        MidiMessage::ControlChange { control, .. } => {
                            if config.out_cc_edit_buffer_dump_req.contains(&control) {
                                // chain up an "edit buffer dump request" message
                                messages = DynIter::new(
                                    make_dump_request(Program::EditBuffer).unwrap().into_iter()
                                        .chain(messages)
                                );
                            }
                        }
                        _ => {}
                    }

                    match midi_out_tx.send(message) {
                        Ok(_) => {}
                        Err(err) => { error!("MIDI OUT error: {}", err); }
                    }
                }
            }
        });
    }

    // midi in -> controller / ui controller / midi out
    {
        let edit_buffer = edit_buffer.clone();
        let dump = dump.clone();
        let ui_controller = ui_controller.clone();
        let config = config.clone();
        let midi_out_tx = state.lock().unwrap().midi_out_tx.clone();
        let ui_event_tx = state.lock().unwrap().ui_event_tx.clone();
        let midi_channel_num = midi_channel_num.clone();
        let detected = state.lock().unwrap().detected.clone();
        tokio::spawn(async move {
            loop {
                let msg = midi_in_rx.recv().await;
                if msg.is_none() {
                    return; // shutdown
                }
                let msg = msg.unwrap();
                trace!("<< {:?}", msg);
                let config = *config.read().unwrap();
                match msg {
                    MidiMessage::ControlChange { channel, control: cc, value } => {
                        let expected_channel = midi_channel_num.load(Ordering::Relaxed);
                        if expected_channel != Channel::all() && channel != expected_channel {
                            // Ignore midi messages sent to a different channel
                            continue;
                        }

                        if config.in_cc_edit_buffer_dump_req.contains(&cc) {
                            // send an "edit buffer dump request"
                            midi_out_tx.send(MidiMessage::ProgramEditBufferDumpRequest);
                        }

                        let edit_buffer = edit_buffer.load();
                        let mut edit_buffer = edit_buffer.lock().unwrap();

                        let control = config.cc_to_control(cc);
                        if control.is_none() {
                            warn!("Control for CC={} not defined!", cc);
                            continue;
                        }
                        let (name, control) = control.unwrap();
                        // Map the control address to a control for the value lookup.
                        // For most controls this will the same control as the one
                        // resolved by CC, for multibyte controls this will be the
                        // head control.
                        let (name, value_control) = control.get_addr()
                            .and_then(|(addr, _)| {
                                config.addr_to_control_vec(addr as usize, false).into_iter().next()
                            })
                            .filter(|(name, control)| {
                                let (_, size) = control.get_addr().unwrap();
                                size > 1
                            })
                            // single byte control, or control without address
                            .unwrap_or_else(|| (name, control));

                        let control_value = edit_buffer.get(name).unwrap();
                        let value = control.value_from_midi(value, control_value);
                        let modified = edit_buffer.set(name, value, MIDI);
                        if modified {
                            // CC from MIDI -> set modified flag
                            set_current_program_modified(
                                &mut edit_buffer,
                                &ui_controller.lock().unwrap(),
                                &ui_event_tx
                            );
                        }
                    },
                    MidiMessage::ProgramChange { channel, program } => {
                        let expected_channel = midi_channel_num.load(Ordering::Relaxed);
                        if expected_channel != Channel::all() && channel != expected_channel {
                            // Ignore midi messages sent to a different channel
                            continue;
                        }
                        let edit_buffer = edit_buffer.load();
                        let dump = dump.load();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let mut ui_controller = ui_controller.lock().unwrap();
                        program_change(&mut dump, &mut edit_buffer,
                                       &mut ui_controller, program, MIDI);
                    }
                    MidiMessage::ProgramEditBufferDump { ver, data } => {
                        // TODO: program name
                        if ver != 0 {
                            error!("Program dump version not supported: {}", ver);
                            continue;
                        }
                        if data.len() != config.program_size {
                            error!("Program size mismatch: expected {}, got {}",
                                  config.program_size, data.len());
                            continue;
                        }
                        program::load_patch_dump_ctrl(
                            &mut edit_buffer.load().lock().unwrap(), data.as_slice(), MIDI);
                    },
                    MidiMessage::ProgramEditBufferDumpRequest => {
                        let edit_buffer = edit_buffer.load();
                        let dump = dump.load();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let ui_controller = ui_controller.lock().unwrap();
                        let messages = program_dump_message(
                            Program::EditBuffer, &mut dump, &mut edit_buffer,
                            &ui_controller, &ui_event_tx, config);
                        for m in messages.unwrap_or_default() {
                            midi_out_tx.send(m);
                        }
                    },
                    MidiMessage::ProgramPatchDump { patch, ver, data } => {
                        if ver != 0 {
                            error!("Program dump version not supported: {}", ver);
                            continue;
                        }
                        if data.len() != config.program_size {
                            error!("Program size mismatch: expected {}, got {}",
                                  config.program_size, data.len());
                            continue;
                        }
                        let edit_buffer = edit_buffer.load();
                        let dump = dump.load();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let current = ui_controller.get("program").unwrap();
                        program::load_patch_dump(&mut dump, patch as usize, data.as_slice(), MIDI);
                        if current > 0 && patch as u16 == (current - 1) {
                            // update edit buffer as well
                            program::load_patch_dump_ctrl(
                                &mut edit_buffer, data.as_slice(), MIDI);
                        }
                        ui_event_tx.send(UIEvent::Modified(patch as usize, false));
                    },
                    MidiMessage::ProgramPatchDumpRequest { patch } => {
                        let edit_buffer = edit_buffer.load();
                        let dump = dump.load();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let ui_controller = ui_controller.lock().unwrap();
                        let messages  = program_dump_message(
                            Program::Number(patch as usize + 1), &mut dump, &mut edit_buffer,
                            &ui_controller, &ui_event_tx, config);
                        for m in messages.unwrap_or_default() {
                            midi_out_tx.send(m);
                        }
                    },
                    MidiMessage::AllProgramsDump { ver, data } => {
                        if ver != 0 {
                            error!("Program dump version not supported: {}", ver);
                            continue;
                        }
                        if data.len() != (config.program_size * config.program_num) {
                            error!("Program size mismatch: expected {}, got {}",
                                  (config.program_size * config.program_num), data.len());
                            continue;
                        }
                        let edit_buffer = edit_buffer.load();
                        let dump = dump.load();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        program::load_all_dump(
                            &mut dump, data.as_slice(), MIDI);
                        for i in 0 .. config.program_num {
                            ui_event_tx.send(UIEvent::Modified(i, false));
                        }
                        // update edit buffer
                        let current = ui_controller.get("program").unwrap();
                        if current > 0 && current as usize <= dump.program_num() {
                            program::load_patch_dump_ctrl(
                                &mut edit_buffer,
                                dump.data(current as usize - 1).unwrap(), MIDI);
                        }
                    },
                    MidiMessage::AllProgramsDumpRequest => {
                        let edit_buffer = edit_buffer.load();
                        let dump = dump.load();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let ui_controller = ui_controller.lock().unwrap();
                        let messages = program_dump_message(
                            Program::All, &mut dump, &mut edit_buffer,
                            &ui_controller, &ui_event_tx, config);
                        for m in messages.unwrap_or_default() {
                            midi_out_tx.send(m);
                        }
                    },
                    MidiMessage::UniversalDeviceInquiryResponse { family, member, ver, .. } => {
                        let hi = if &ver[0 .. 1] == "0" { &ver[1 ..= 1] } else { &ver[0 ..= 1] };
                        let lo = &ver[2 ..= 3];
                        let ver = format!("{}.{}", hi, lo);
                        let name = config_for_id(family, member)
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| format!("Unknown ({:04x}:{:04x})", family, member));

                        detected.store(Some(Arc::new(DetectedDevVersion {
                            name,
                            ver
                        })));
                        ui_event_tx.send(UIEvent::NewMidiConnection);
                    }

                    // pretend we're a POD
                    MidiMessage::UniversalDeviceInquiry { channel } => {
                        let expected_channel = midi_channel_num.load(Ordering::Relaxed);
                        if channel != expected_channel && channel != Channel::all() {
                            // Ignore midi messages sent to a different channel,
                            // but answer messages sent to "all" as "all"
                            return;
                        }
                        let res = MidiMessage::UniversalDeviceInquiryResponse {
                            channel,
                            family: config.family,
                            member: config.member,
                            ver: String::from("0223")
                        };
                        midi_out_tx.send(res);
                    }

                    _ => {
                        warn!("Unhandled MIDI message: {:?}", msg);
                    }
                }
            }
        });
    }

    // ---------------------------------------------------------

    // controller -> gui
    {
        let edit_buffer = edit_buffer.clone();
        let ui_controller = ui_controller.clone();

        let mut objects = ObjectList::default();
        let mut callbacks = Callbacks::new();
        let mut rx = None;
        let mut ui_rx = ui_controller.lock().unwrap().subscribe();
        let mut names_rx = None;

        let transfer_up_sem = Arc::new(atomic::AtomicI32::new(0));
        let transfer_down_sem = Arc::new(atomic::AtomicI32::new(0));

        // This is a cache of the current page number of the whole of the glib idle callback
        let mut current_page = 0usize;

        glib::idle_add_local(move || {
            if rx.is_none() {
                let edit_buffer = edit_buffer.load();
                let dump = dump.load();

                let edit_buffer = edit_buffer.lock().unwrap();
                rx = Some(edit_buffer.subscribe());

                let dump = dump.lock().unwrap();
                names_rx = Some(dump.subscribe_to_name_updates());

                let state = state.lock().unwrap();
                objects = &state.interface.objects + &ui_objects;
            }

            let mut processed = false;
            match rx.as_mut().unwrap().try_recv() {
                Ok(Event { key: name, .. }) => {
                    processed = true;
                    let vec = callbacks.get_vec(&name);
                    match vec {
                        None => { warn!("No GUI callback for '{}'", &name); },
                        Some(vec) => for cb in vec {
                            cb()
                        }
                    }
                    let edit_buffer = edit_buffer.load();
                    let edit_buffer = edit_buffer.lock().unwrap();
                    animate(&objects, &name, edit_buffer.get(&name).unwrap());
                },
                _ => {}
            }
            match ui_rx.try_recv() {
                Ok(Event { key: name, .. }) => {
                    processed = true;
                    let vec = ui_callbacks.get_vec(&name);
                    match vec {
                        None => { warn!("No GUI callback for '{}'", &name); },
                        Some(vec) => for cb in vec {
                            cb()
                        }
                    }
                    let val = ui_controller.get(&name).unwrap();
                    animate(&objects, &name, val);

                    if name == "program" {
                        current_page = if val > 0 { val as usize - 1 } else { 0usize };
                    }
                },
                _ => {}
            }
            match ui_event_rx.try_recv() {
                Ok(event) => {
                    processed = true;
                    match event {
                        UIEvent::MidiTx => {
                            transfer_icon_up.set_opacity(1.0);
                            transfer_up_sem.fetch_add(1, Ordering::SeqCst);
                            {
                                let transfer_icon_up = transfer_icon_up.clone();
                                let transfer_up_sem = Arc::clone(&transfer_up_sem);
                                glib::timeout_add_local_once(
                                    Duration::from_millis(500),
                                    move || {
                                        let v = transfer_up_sem.fetch_add(-1, Ordering::SeqCst);
                                        if v <= 1 {
                                            transfer_icon_up.set_opacity(0.0);
                                        }
                                    });
                            }
                        }
                        UIEvent::MidiRx => {
                            transfer_icon_down.set_opacity(1.0);
                            transfer_down_sem.fetch_add(1, Ordering::SeqCst);
                            {
                                let transfer_icon_down = transfer_icon_down.clone();
                                let transfer_down_sem = Arc::clone(&transfer_down_sem);
                                glib::timeout_add_local_once(
                                    Duration::from_millis(500),
                                    move || {
                                        let v = transfer_down_sem.fetch_add(-1, Ordering::SeqCst);
                                        if v <= 1 {
                                            transfer_icon_down.set_opacity(0.0);
                                        }
                                    });
                            }
                        }
                        UIEvent::NewMidiConnection => {
                            let state = state.lock().unwrap();
                            let midi_in_name = state.midi_in_name.as_ref();
                            let midi_out_name = state.midi_out_name.as_ref();
                            let name = {
                                let config_name = &state.config.read().unwrap().name;
                                let (detected_name, detected_ver) = state.detected.load()
                                    .as_ref()
                                    .map(|d| (d.name.clone(), d.ver.clone()))
                                    .unwrap_or_else(|| (String::new(), String::new()));
                                match (&detected_name, config_name) {
                                    (a, b) if a.is_empty() => {
                                        b.clone()
                                    },
                                    (a, b) if a == b => {
                                        format!("{} {}", detected_name, detected_ver)
                                    },
                                    _ => {
                                        format!("{} {} as {}", detected_name, detected_ver, config_name)
                                    }
                                }
                            };
                            let subtitle = match (midi_in_name, midi_out_name) {
                                (None, _) | (_, None) => {
                                    "no device connected".to_string()
                                }
                                (Some(a), Some(b)) if a == b => {
                                    format!("{}", a)
                                }
                                (Some(a), Some(b)) => {
                                    format!("i: {} / o: {}", a, b)
                                }
                            };
                            let subtitle = format!("{} @ {}", name, subtitle);

                            header_bar.set_subtitle(Some(&subtitle));
                            // update midi channel number
                            midi_channel_num.store(state.midi_channel_num, Ordering::Relaxed);
                        }
                        UIEvent::NewEditBuffer => {
                            let state = state.lock().unwrap();

                            rx = None;

                            device_box.foreach(|w| device_box.remove(w));
                            device_box.add(&state.interface.widget);
                            callbacks = state.interface.callbacks.clone();

                            {
                                // Another ugly hack: to initialize the UI, it is not enough
                                // to animate() the init_controls, since the controls do
                                // emit other (synthetic or otherwise) animate() calls in their
                                // wiring. So, we both animate() as part of init_module()
                                // (needed to hide most controls that hide before first show)
                                // and defer an init_module_controls() call that needs to happen
                                // after the rx is subscribed to again!
                                let config = state.config.clone();
                                let edit_buffer = edit_buffer.load().clone();
                                glib::idle_add_local_once(move || {
                                    let config = config.read().unwrap();
                                    let edit_buffer = edit_buffer.lock().unwrap();
                                    init_module_controls(&config, &edit_buffer)
                                        .unwrap_or_else(|err| error!("{}", err));
                                });
                            }

                            // Update UI from state after device change
                            update_ui_from_state(&state, &mut ui_controller.lock().unwrap());

                            let grid = ui.object::<gtk::Grid>("program_grid").unwrap();
                            ObjectList::from_widget(&grid)
                                .objects_by_type::<ProgramGrid>()
                                .for_each(|p| {
                                    grid.remove(p);
                                    // This instance of ProgramGrid gets dropped, but the
                                    // ad-hoc signalling using "group-changed" still sees
                                    // its widgets as part of the radio group (not dropped
                                    // immediately?) and there will be no final signal to
                                    // reset the group/signal handlers to remove the ones
                                    // that became invalid.
                                    // TODO: add "on destroy" clean up to wired RadioButtons
                                    //       as a fix? In  the meantime, this hack will do...
                                    p.join_radio_group(Option::<&gtk::RadioButton>::None);
                                });

                            let program_num = state.config.read().unwrap().program_num;
                            let g = ProgramGrid::new(program_num);
                            grid.attach(&g, 0, 1, 2, 18);
                            g.show_all();

                            // join the main program radio group
                            let r = ui.object::<gtk::RadioButton>("program").unwrap();
                            g.join_radio_group(Some(&r));
                            r.emit_by_name::<()>("group-changed", &[]);

                            // show "open" button in the titlebar?
                            let open_button = ui.object::<gtk::Button>("open_button").unwrap();
                            if g.num_pages() > 1 {
                                open_button.show();
                            } else {
                                open_button.hide();
                            }

                            program_grid.store(Arc::new(g));
                        }
                        UIEvent::NewDevice => {
                            // connected to a possibly new  device, perform a new device ping
                            let state = state.lock().unwrap();
                            new_device_ping(&state);
                        }
                        UIEvent::Modified(page, modified) => {
                            // patch index is 1-based
                            (*program_grid.load()).set_program_modified(page + 1, modified);
                        },
                        UIEvent::Panic => {
                            let tooltip = if sentry_enabled {
                                Some("\
                                Something broke in the app and one of its internal \
                                processing threads crashed. You can check the logs to see what \
                                exactly happened. The error has been reported to the cloud.\
                                ")
                            } else { None };
                            objects.obj_by_name("panic_indicator").ok()
                                .and_then(|obj| obj.dynamic_cast::<gtk::Widget>().ok())
                                .map(|widget| {
                                    widget.set_visible(true);
                                    if tooltip.is_some() {
                                        widget.set_tooltip_text(tooltip);
                                    }
                                });
                        },
                        UIEvent::Quit => {
                            // application is being closed, perform clean-up
                            // that needs to happen inside the GTK thread...

                            let window: gtk::Window = ui.object("ui_win").unwrap();

                            // detach the program buttons so that there are no
                            // "GTK signal handler not found" errors when SignalHandler
                            // objects are dropped...
                            let r = ui.object::<gtk::RadioButton>("program").unwrap();
                            let g = program_grid.load_full();
                            g.join_radio_group(Option::<&gtk::RadioButton>::None);
                            r.emit_by_name::<()>("group-changed", &[]);

                            // quit
                            gtk::main_quit();
                        }
                    }

                }
                _ => {}
            }
            match names_rx.as_mut().unwrap().try_recv() {
                Ok(Event { key: idx, .. }) => {
                    processed = true;
                    let name = dump.load().lock().unwrap().name(idx).unwrap_or_default();
                    // program button index is 1-based
                    (*program_grid.load()).set_program_name(idx + 1, &name);
                },
                _ => {}
            }

            // an ugly hack to stop the app from consuming 100% cpu
            if !processed {
                thread::sleep(Duration::from_millis(100));
            }
            Continue(true)
        });

    }

    // show the window and do init stuff...
    window.show_all();
    window.resize(1, 1);

    debug!("starting gtk main loop");
    gtk::main();

    Ok(())
}
