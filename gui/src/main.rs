mod opts;
mod util;
mod settings;
mod program_button;
mod panic;
mod registry;

use anyhow::*;

use pod_gtk::*;
use pod_gtk::gtk::prelude::*;
use pod_core::pod::*;
use pod_core::controller::{Controller, ControllerStoreExt};
use pod_core::program;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{AbstractControl, Button, Config, Control};
use pod_core::config::{configs, GUI, MIDI, register_config, UNSET};
use crate::opts::*;
use pod_core::midi::{Channel, MidiMessage};
use tokio::sync::{broadcast, mpsc, oneshot};
use std::thread;
use pod_core::store::{Event, Store};
use core::result::Result::Ok;
use std::collections::HashMap;
use std::ops::Add;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use maplit::*;
use crate::settings::*;

pub enum UIEvent {
    NewMidiConnection,
    NewEditBuffer,
    MidiTx,
    MidiRx,
    Modified(usize, bool),
    Panic
}

pub struct State {
    pub midi_in_name: Option<String>,
    pub midi_in_cancel: Option<oneshot::Sender<()>>,

    pub midi_out_name: Option<String>,
    pub midi_out_cancel: Option<oneshot::Sender<()>>,

    pub midi_in_tx: mpsc::UnboundedSender<MidiMessage>,
    pub midi_out_tx: broadcast::Sender<MidiMessage>,
    pub ui_event_tx: mpsc::UnboundedSender<UIEvent>,

    pub midi_channel_num: u8,
    pub config: Option<&'static Config>,
    pub edit_buffer: Option<EditBuffer>
}

use pod_core::model::SwitchControl;
static UI_CONTROLS: Lazy<HashMap<String, Control>> = Lazy::new(|| {
    convert_args!(hashmap!(
        "program" => SwitchControl::default(),
        "program:prev" => SwitchControl::default(),
        "program_num" => SwitchControl::default(),
        "load_button" => Button::default(),
        "load_patch_button" => Button::default(),
        "load_all_button" => Button::default(),
        "store_button" => Button::default(),
        "store_patch_button" => Button::default(),
        "store_all_button" => Button::default(),
    ))
});


fn init_all(config: &Config, controller: Arc<Mutex<Controller>>, objs: &ObjectList) -> () {
    for name in &config.init_controls {
        animate(objs, &name, controller.get(&name).unwrap());
    }
}

pub fn set_midi_in_out(state: &mut State, midi_in: Option<MidiIn>, midi_out: Option<MidiOut>,
                       midi_channel: u8, config: Option<&'static Config>) {
    state.midi_in_cancel.take().map(|cancel| cancel.send(()));
    state.midi_out_cancel.take().map(|cancel| cancel.send(()));

    if midi_in.is_none() || midi_out.is_none() || config.is_none() {
        warn!("Not starting MIDI because in/out is None");
        state.midi_in_name = None;
        state.midi_in_cancel = None;
        state.midi_out_name = None;
        state.midi_out_cancel = None;
        state.ui_event_tx.send(UIEvent::NewMidiConnection);
        state.midi_channel_num = 0;
        state.config = None;
        return;
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

    let config_changed = match (config, state.config) {
        (Some(_), None) => { true }
        (None, Some(_)) => { true }
        (Some(a), Some(b)) => { *a != *b }
        _ => { false }
    };
    if config_changed {
        // config changed, update config & edit buffer
        state.config = config;
        state.edit_buffer = Some(EditBuffer::new(state.config.as_ref().unwrap()));
        state.ui_event_tx.send(UIEvent::NewEditBuffer);

    }


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
                    Ok(msg) = midi_out_rx.recv() => {
                        let bytes = msg.to_bytes();
                        midi_out.send(&bytes);
                        ui_event_tx.send(UIEvent::MidiTx);
                    }
                    _ = &mut out_cancel_rx => {
                        return;
                    }
                }
            }
        });
    }

    // Request all programs dump from the POD device
    state.midi_out_tx.send(MidiMessage::AllProgramsDumpRequest).unwrap();
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

    let program = program as usize;
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
    ui_controller: &Controller, ui_event_tx: &mpsc::UnboundedSender<UIEvent>
) -> Option<MidiMessage> {
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
            Some(MidiMessage::ProgramEditBufferDump {
                ver: 0,
                data: program::store_patch_dump_ctrl(&edit_buffer)
            })
        }
        Program::Current if !cur_program_valid => { None }
        Program::Current => {
            let patch = cur_program - 1;
            dump.set_modified(patch, false);
            ui_event_tx.send(UIEvent::Modified(patch, false));

            Some(MidiMessage::ProgramPatchDump {
                patch: patch as u8,
                ver: 0,
                data: program::store_patch_dump(&dump, patch)
            })
        }
        Program::Number(n) => {
            let patch = n - 1;
            dump.set_modified(patch, false);
            ui_event_tx.send(UIEvent::Modified(patch, false));

            Some(MidiMessage::ProgramPatchDump {
                patch: patch as u8,
                ver: 0,
                data: program::store_patch_dump(&dump, patch)
            })
        }
        Program::All => {
            dump.set_all_modified(false);
            for i in 0 .. dump.program_num() {
                ui_event_tx.send(UIEvent::Modified(i, false));
            }

            Some(MidiMessage::AllProgramsDump {
                ver: 0,
                data: program::store_all_dump(&dump)
            })
        }
    }

}


fn set_current_program_modified(edit: &mut EditBuffer, ui_controller: &Controller, state: &State) {
    let cur_program = ui_controller.get(&"program").unwrap() as usize;
    let program_num = ui_controller.get(&"program_num").unwrap() as usize;
    let cur_program_valid = cur_program > 0 && cur_program < program_num;
    if cur_program_valid {
        let current_page = cur_program - 1;
        state.ui_event_tx.send(UIEvent::Modified(current_page, true));

    }
    edit.set_modified(true);
}

use result::prelude::*;
use pod_core::dump::ProgramsDump;
use pod_core::edit::EditBuffer;
use pod_gtk::gtk::gdk;
use crate::panic::wire_panic_indicator;
use crate::program_button::ProgramButtons;
use crate::registry::{module_for_config, register_module};


#[tokio::main]
async fn main() -> Result<()> {
    let _guard = sentry::init((option_env!("SENTRY_DSN"), sentry::ClientOptions {
        release: Some(env!("GIT_VERSION").into()),
        ..Default::default()
    }));
    let sentry_enabled = _guard.is_enabled();
    simple_logger::init()?;

    let opts: Opts = Opts::parse();

    let (midi_in_tx, mut midi_in_rx) = mpsc::unbounded_channel::<MidiMessage>();
    let (midi_out_tx, mut midi_out_rx) = broadcast::channel::<MidiMessage>(16);
    let (ui_event_tx, mut ui_event_rx) = mpsc::unbounded_channel::<UIEvent>();

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    register_module(pod_mod_pod2::module());
    let config = configs().iter().next().unwrap();
    let module = module_for_config(config).unwrap();

    let state = Arc::new(Mutex::new(State {
        midi_in_name: None,
        midi_in_cancel: None,
        midi_out_name: None,
        midi_out_cancel: None,
        midi_in_tx,
        midi_out_tx,
        ui_event_tx,
        midi_channel_num: 0,
        config: None,
        edit_buffer: None
    }));

    // autodetect/open midi
    let autodetect = match (&opts.input, &opts.output) {
        (None, None) => true,
        _ => false
    };
    let (mut midi_in, mut midi_out, midi_channel, detected_config) =
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
            let mut midi_in =
                opts.input.map(MidiIn::new_for_address).invert()?;
            let midi_out =
                opts.output.map(MidiOut::new_for_address).invert()?;
            let midi_channel = opts.channel.unwrap_or(0);

            // TODO: specify config on the command line
            (midi_in, midi_out, midi_channel, None)
        };

    // moving this to below the panic handler so that early crashed
    // in the midi thread are shown in the UI
    //set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out, midi_channel);

    // midi channel number
    let midi_channel_num = Arc::new(AtomicU8::new(0));

    // edit buffer
    let edit_buffer = Arc::new(Mutex::new(EditBuffer::new(&config)));
    let controller = edit_buffer.lock().unwrap().controller();

    // programs
    let dump = Arc::new(Mutex::new(ProgramsDump::new(&config)));

    let mut callbacks = Callbacks::new();

    let ui = gtk::Builder::from_string(include_str!("ui.glade"));
    let ui_objects = ObjectList::new(&ui);
    let ui_controller = Arc::new(Mutex::new(Controller::new((*UI_CONTROLS).clone())));
    pod_gtk::wire(ui_controller.clone(), &ui_objects, &mut callbacks)?;
    let mut program_buttons = ProgramButtons::new(&ui_objects);

    let title = format!("POD UI {}", env!("GIT_VERSION"));

    let window: gtk::Window = ui.object("ui_win").unwrap();
    window.set_title(&title);
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    let transfer_icon_up: gtk::Label = ui.object("transfer_icon_up").unwrap();
    let transfer_icon_down: gtk::Label = ui.object("transfer_icon_down").unwrap();
    transfer_icon_up.set_opacity(0.0);
    transfer_icon_down.set_opacity(0.0);
    let header_bar: gtk::HeaderBar = ui.object("header_bar").unwrap();

    wire_settings_dialog(state.clone(), &ui);
    wire_panic_indicator(state.clone());

    set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out, midi_channel, Some(&config));

    let css = gtk::CssProvider::new();
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Error initializing GTK CSS provider"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    let pod_ui = module.widget();
    let app_grid: gtk::Box = ui.object("app_grid").unwrap();
    app_grid.add(&pod_ui);

    module.wire(&config, edit_buffer.clone(), &mut callbacks)?;
    //ui_controller.lock().unwrap().set("program_num", module.config().program_num as u16, UNSET);
    let objects = ui_objects + module.objects();

    // ---------------------------------------------------------
    edit_buffer.lock().unwrap().start_thread();

    // midi ----------------------------------------------------

    // controller / ui controller / midi reply -> midi out
    {
        let state = state.clone();
        let controller = controller.clone();
        let edit_buffer = edit_buffer.clone();
        let ui_controller = ui_controller.clone();
        let dump = dump.clone();
        let mut rx = controller.lock().unwrap().subscribe();
        let mut ui_rx = ui_controller.lock().unwrap().subscribe();
        let midi_out_tx = state.lock().unwrap().midi_out_tx.clone();
        let midi_channel_num = midi_channel_num.clone();
        tokio::spawn(async move {
            let make_cc = |name: &str| -> Option<MidiMessage> {
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

                let channel = midi_channel_num.load(Ordering::Relaxed);
                let channel = if channel == Channel::all() { 0 } else { channel };
                let value = control.value_to_midi(value);
                Some(MidiMessage::ControlChange { channel, control: cc.unwrap(), value })
            };
            let make_pc = || {
                let channel = midi_channel_num.load(Ordering::Relaxed);
                let channel = if channel == Channel::all() { 0 } else { channel };
                let ui_controller = ui_controller.lock().unwrap();
                let v = ui_controller.get(&"program").unwrap();
                Some(MidiMessage::ProgramChange { channel, program: v as u8 })
            };
            let make_dump_request = |program: Program| {
                let ui_controller = ui_controller.lock().unwrap();
                match program {
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
                }
            };
            let make_dump = |program: Program| {
                let mut edit_buffer = edit_buffer.lock().unwrap();
                let mut dump = dump.lock().unwrap();
                let ui_controller = ui_controller.lock().unwrap();
                let state = state.lock().unwrap();
                program_dump_message(program, &mut dump, &mut edit_buffer,
                                     &ui_controller, &state.ui_event_tx)
            };

            loop {
                let mut message: Option<MidiMessage> = None;
                let mut origin: u8 = UNSET;
                tokio::select! {
                  Ok(Event { key: name, origin: o, .. }) = rx.recv() => {
                        message = make_cc(&name);
                        origin = o;
                    }
                  Ok(Event { key, origin: o, .. }) = ui_rx.recv() => {
                        message = match key.as_str() {
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
                }
                if origin == MIDI || message.is_none() {
                    continue;
                }
                let send_buffer = match message {
                    Some(MidiMessage::ControlChange { ..}) => {
                        // CC from GUI layer -> set modified flag
                        set_current_program_modified(
                            &mut edit_buffer.lock().unwrap(),
                            &ui_controller.lock().unwrap(),
                            &state.lock().unwrap()
                        );
                        false
                    }
                    Some(MidiMessage::ProgramChange { program, .. }) => {
                        let mut dump = dump.lock().unwrap();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut ui_controller = ui_controller.lock().unwrap();
                        program_change(&mut dump, &mut edit_buffer, &mut ui_controller, program, GUI)
                    }
                    _ => { false }
                };

                // If the selected program was modified, Line6 Edit doesn't send a
                // PC followed by an edit buffer dump, which would be logical, but sends an
                // edit buffer dump only. Indeed, if we send PC first and then the edit buffer
                // dump, Pod 2.0 gets all confused and switches to a completely different
                // program altogether. So, following Line6 Edit we only sent the edit buffer dump!
                if send_buffer {
                    message = make_dump(Program::EditBuffer);
                }

                match midi_out_tx.send(message.unwrap()) {
                    Ok(_) => {}
                    Err(err) => { error!("MIDI OUT error: {}", err); }
                }
            }
        });
    }

    // midi in -> controller / ui controller / midi out
    {
        let edit_buffer = edit_buffer.clone();
        let controller = controller.clone();
        let ui_controller = ui_controller.clone();
        let config = config.clone();
        let midi_out_tx = state.lock().unwrap().midi_out_tx.clone();
        let dump = dump.clone();
        let state = state.clone();
        let midi_channel_num = midi_channel_num.clone();
        tokio::spawn(async move {
            loop {
                let msg = midi_in_rx.recv().await;
                if msg.is_none() {
                    return; // shutdown
                }
                let msg = msg.unwrap();
                trace!("<< {:?}", msg);
                match msg {
                    MidiMessage::ControlChange { channel, control: cc, value } => {
                        let expected_channel = midi_channel_num.load(Ordering::Relaxed);
                        if expected_channel != Channel::all() && channel != expected_channel {
                            // Ignore midi messages sent to a different channel
                            continue;
                        }
                        let mut controller = controller.lock().unwrap();

                        let control = config.cc_to_control(cc);
                        if control.is_none() {
                            warn!("Control for CC={} not defined!", cc);
                            continue;
                        }
                        let (name, control) = control.unwrap();
                        let value = control.value_from_midi(value);
                        let modified = controller.set(name, value, MIDI);
                        if modified {
                            // CC from MIDI -> set modified flag
                            set_current_program_modified(
                                &mut edit_buffer.lock().unwrap(),
                                &ui_controller.lock().unwrap(),
                                &state.lock().unwrap()
                            );
                        }
                    },
                    MidiMessage::ProgramChange { channel, program } => {
                        let expected_channel = midi_channel_num.load(Ordering::Relaxed);
                        if expected_channel != Channel::all() && channel != expected_channel {
                            // Ignore midi messages sent to a different channel
                            continue;
                        }
                        let mut ui_controller = ui_controller.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let mut edit_buffer = edit_buffer.lock().unwrap();
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
                            &mut edit_buffer.lock().unwrap(), data.as_slice(), MIDI);
                    },
                    MidiMessage::ProgramEditBufferDumpRequest => {
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let ui_controller = ui_controller.lock().unwrap();
                        let state = state.lock().unwrap();
                        let msg = program_dump_message(
                            Program::EditBuffer, &mut dump, &mut edit_buffer,
                            &ui_controller, &state.ui_event_tx);
                        midi_out_tx.send(msg.unwrap());
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
                        let mut dump = dump.lock().unwrap();
                        let current = ui_controller.get("program").unwrap();
                        program::load_patch_dump(&mut dump, patch as usize, data.as_slice(), MIDI);
                        if current > 0 && patch as u16 == (current - 1) {
                            // update edit buffer as well
                            program::load_patch_dump_ctrl(
                                &mut edit_buffer.lock().unwrap(), data.as_slice(), MIDI);
                        }
                        state.lock().unwrap()
                            .ui_event_tx.send(UIEvent::Modified(patch as usize, false));
                    },
                    MidiMessage::ProgramPatchDumpRequest { patch } => {
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let ui_controller = ui_controller.lock().unwrap();
                        let state = state.lock().unwrap();
                        let msg = program_dump_message(
                            Program::Number(patch as usize + 1), &mut dump, &mut edit_buffer,
                            &ui_controller, &state.ui_event_tx);
                        midi_out_tx.send(msg.unwrap());
                    },
                    MidiMessage::AllProgramsDump { ver, data } => {
                        let mut dump = dump.lock().unwrap();
                        if ver != 0 {
                            error!("Program dump version not supported: {}", ver);
                            continue;
                        }
                        if data.len() != (config.program_size * config.program_num) {
                            error!("Program size mismatch: expected {}, got {}",
                                  (config.program_size * config.program_num), data.len());
                            continue;
                        }
                        program::load_all_dump(
                            &mut dump, data.as_slice(), MIDI);
                        let state = state.lock().unwrap();
                        for i in 0 .. config.program_num {
                            state.ui_event_tx.send(UIEvent::Modified(i, false));
                        }
                        // update edit buffer
                        let current = ui_controller.get("program").unwrap();
                        if current > 0 && current as usize <= dump.program_num() {
                            program::load_patch_dump_ctrl(
                                &mut edit_buffer.lock().unwrap(),
                                dump.data(current as usize - 1).unwrap(), MIDI);
                        }
                    },
                    MidiMessage::AllProgramsDumpRequest => {
                        let mut edit_buffer = edit_buffer.lock().unwrap();
                        let mut dump = dump.lock().unwrap();
                        let ui_controller = ui_controller.lock().unwrap();
                        let state = state.lock().unwrap();
                        let msg = program_dump_message(
                            Program::All, &mut dump, &mut edit_buffer,
                            &ui_controller, &state.ui_event_tx);
                        midi_out_tx.send(msg.unwrap());
                    },

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

    /*
    // raw -----------------------------------------------------

    // controller -> raw
    {
        let raw = raw.clone();
        let controller = controller.clone();
        let config = config.clone();
        let state = state.clone();
        let mut rx = controller.lock().unwrap().subscribe();
        tokio::spawn(async move {
            loop {
                let mut name: String = String::new();
                let mut signal: Signal = Signal::None;
                match rx.recv().await {
                    Ok(Event { key: n, signal: s, .. }) => {
                        name = n;
                        signal = s;
                    }
                    Err(e) => {
                        error!("Error in 'controller -> raw' rx: {}", e)
                    }
                }

                let controller = controller.lock().unwrap();
                let mut raw = raw.lock().unwrap();
                let control_config = controller.get_config(&name);
                if control_config.is_none() {
                    warn!("Control {:?} not found!", &name);
                    continue;
                }

                let (value, origin) = controller.get_origin(&name).unwrap();
                if origin != GUI {
                    continue;
                }

                let control_config = control_config.unwrap();
                let addr= control_config.get_addr();
                if addr.is_none() {
                    continue; // skip virtual controls
                }
                let (addr, bytes) = control_config.get_addr().unwrap();
                let modified = match bytes {
                    1 => {
                        raw.set_full(addr as usize, value as u8, GUI, signal)
                    }
                    2 => {
                        let bits = match control_config {
                            Control::RangeControl(RangeControl{ config: RangeConfig::Long { bits }, .. }) => bits,
                            _ => &[0u8, 0u8]
                        };

                        let b1 = value >> bits[1];
                        let b2 = value & ((1 << bits[1]) - 1);

                        // For "delay time" knob, Line6 Edit always sends the
                        // "time 1 fine cc 62" first, and then "time 1 coarse
                        // cc 30". It also expects to receive them in the same
                        // order, so sending only cc 62 on change is like also
                        // sending cc 30 = 0!
                        // So, 1) set LSB first and then MSB and 2) always
                        // force-set.
                        // ---
                        // NOTE: Even that is not enough, though! Line6 Edit
                        // 3.06 actually will discard cc 62 after getting
                        // cc 30, so, sending cc 62 is like sending cc 30 = 0
                        // and sending cc 30 is like sending cc 62 = 0 !!! ;(
                        // Still it's better so send cc 30 (coarse) overriding
                        // cc 62 (fine) than the other way around.
                        let c2 = raw.set_full((addr+1) as usize, b2 as u8, GUI, Signal::Force);
                        let c1 = raw.set_full(addr as usize, b1 as u8, GUI, Signal::Force);
                        c1 || c2
                    }
                    n => {
                        error!("Unsupported control size in bytes: {}", n);
                        false
                    }
                };
                if modified {
                    trace!("modified triggered by {:?}", name);
                    state.lock().unwrap().ui_event_tx.send(UIEvent::Modified(raw.page, true));
                }
            }
        });
    }
*/

    // ---------------------------------------------------------

    // controller -> gui
    {
        let edit_buffer = edit_buffer.clone();
        let controller = controller.clone();
        let ui_controller = ui_controller.clone();
        let objects = objects.clone();

        let mut rx = {
            let controller = controller.lock().unwrap();
            controller.subscribe()
        };
        let mut ui_rx = {
            let ui_controller = ui_controller.lock().unwrap();
            ui_controller.subscribe()
        };
        let mut names_rx = dump.lock().unwrap().subscribe_to_name_updates();

        let transfer_up_sem = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let transfer_down_sem = Arc::new(std::sync::atomic::AtomicI32::new(0));

        // This is a cache of the current page number of the whole of the glib idle callback
        let mut current_page = 0usize;

        glib::idle_add_local(move || {
            let mut processed = false;
            match rx.try_recv() {
                Ok(Event { key: name, .. }) => {
                    processed = true;
                    let vec = callbacks.get_vec(&name);
                    match vec {
                        None => { warn!("No GUI callback for '{}'", &name); },
                        Some(vec) => for cb in vec {
                            cb()
                        }
                    }
                    animate(&objects, &name, controller.get(&name).unwrap());
                },
                _ => {}
            }
            match ui_rx.try_recv() {
                Ok(Event { key: name, .. }) => {
                    processed = true;
                    let vec = callbacks.get_vec(&name);
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

                            header_bar.set_subtitle(Some(&subtitle));
                            // update midi channel number
                            midi_channel_num.store(state.midi_channel_num, Ordering::Relaxed);
                        }
                        UIEvent::NewEditBuffer => {
                            let mut edit_buffer = edit_buffer.lock();
                            
                        }
                        UIEvent::Modified(page, modified) => {
                            // patch index is 1-based
                            program_buttons.set_modified(page + 1, modified);
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
                        }
                    }

                }
                _ => {}
            }
            match names_rx.try_recv() {
                Ok(Event { key: idx, .. }) => {
                    processed = true;
                    let name = dump.lock().unwrap().name(idx).unwrap_or_default();
                    // program button index is 1-based
                    if let Some(button) = program_buttons.get(idx + 1) {
                        button.set_name_label(&name);
                    }
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

    init_all(&config, controller.clone(), &objects);
    module.init(&config, edit_buffer)?;

    debug!("starting gtk main loop");
    gtk::main();

    Ok(())
}
