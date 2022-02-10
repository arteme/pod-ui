mod opts;
mod util;
mod settings;
mod program_button;

use anyhow::*;

use pod_gtk::*;
use pod_gtk::gtk::prelude::*;
use pod_core::pod::*;
use pod_core::controller::Controller;
use pod_core::program;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{AbstractControl, Button, Config, Control, RangeConfig, RangeControl};
use pod_core::config::{GUI, MIDI, register_config, UNSET};
use crate::opts::*;
use pod_core::midi::MidiMessage;
use tokio::sync::{broadcast, mpsc, oneshot};
use core::time;
use std::thread;
use pod_core::raw::Raw;
use pod_core::store::{Event, Signal, Store};
use core::result::Result::Ok;
use std::collections::HashMap;
use std::future::Future;
use once_cell::sync::Lazy;
use std::sync::atomic::Ordering;
use std::time::Duration;
use maplit::*;
use crate::settings::*;

pub enum UIEvent {
    NewMidiConnection,
    MidiTx,
    MidiRx,
    Modified(usize)
}

pub struct State {
    pub midi_in_name: Option<String>,
    pub midi_in_cancel: Option<oneshot::Sender<()>>,

    pub midi_out_name: Option<String>,
    pub midi_out_cancel: Option<oneshot::Sender<()>>,

    pub midi_in_tx: mpsc::UnboundedSender<MidiMessage>,
    pub midi_out_tx: broadcast::Sender<MidiMessage>,
    pub ui_event_tx: mpsc::UnboundedSender<UIEvent>,
}

use pod_core::model::SwitchControl;
static UI_CONTROLS: Lazy<HashMap<String, Control>> = Lazy::new(|| {
    convert_args!(hashmap!(
        "program" => SwitchControl::default(),
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

pub fn set_midi_in_out(state: &mut State, midi_in: Option<MidiIn>, midi_out: Option<MidiOut>) {
    state.midi_in_cancel.take().map(|cancel| cancel.send(()));
    state.midi_out_cancel.take().map(|cancel| cancel.send(()));

    if midi_in.is_none() || midi_out.is_none() {
        warn!("Not starting MIDI because in/out is None");
        state.midi_in_name = None;
        state.midi_in_cancel = None;
        state.midi_out_name = None;
        state.midi_out_cancel = None;
        state.ui_event_tx.send(UIEvent::NewMidiConnection);
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

fn program_change(raw: Arc<Mutex<Raw>>, program: u8, _origin: u8) {
    let mut raw = raw.lock().unwrap();

    // In case of program change, always send a signal that the data change is coming
    // from MIDI so that the GUI gets updated, but the MIDI does not
    raw.set_page_signal(program as usize - 1, MIDI);
}

use result::prelude::*;
use pod_core::names::ProgramNames;
use crate::program_button::ProgramButtons;
use crate::UIEvent::MidiTx;


#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init()?;

    let opts: Opts = Opts::parse();

    let (midi_in_tx, mut midi_in_rx) = mpsc::unbounded_channel::<MidiMessage>();
    let (midi_out_tx, mut midi_out_rx) = broadcast::channel::<MidiMessage>(16);
    let (ui_event_tx, mut ui_event_rx) = mpsc::unbounded_channel::<UIEvent>();

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let module = pod_mod_pod2::module();
    let config = module.config();

    let state = Arc::new(Mutex::new(State {
        midi_in_name: None,
        midi_in_cancel: None,
        midi_out_name: None,
        midi_out_cancel: None,
        midi_in_tx,
        midi_out_tx,
        ui_event_tx
    }));

    // register POD 2.0 module
    register_config(&config);

    // autodetect/open midi
    let autodetect = match (&opts.input, &opts.output) {
        (None, None) => true,
        _ => false
    };
    let (mut midi_in, mut midi_out) = if autodetect {
        match pod_core::pod::autodetect().await {
            Ok((midi_in, midi_out)) => {
                (Some(midi_in), Some(midi_out))
            }
            Err(err) => {
                error!("MIDI autodetect failed: {}", err);
                (None, None)
            }
        }
    } else {
        let mut midi_in =
            opts.input.map(MidiIn::new_for_address).invert()?;
        let midi_out =
            opts.output.map(MidiOut::new_for_address).invert()?;

        (midi_in, midi_out)
    };

    set_midi_in_out(&mut state.lock().unwrap(), midi_in, midi_out);

    let raw = Arc::new(Mutex::new(Raw::new(config.program_size, config.program_num)));
    let controller = Arc::new(Mutex::new(Controller::new(config.controls.clone())));
    let program_names =  Arc::new(Mutex::new(ProgramNames::new(&config)));

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

    let pod_ui = module.widget();
    let app_grid: gtk::Box = ui.object("app_grid").unwrap();
    app_grid.add(&pod_ui);

    module.wire(controller.clone(), raw.clone(), &mut callbacks)?;
    let objects = ui_objects + module.objects();

    // midi ----------------------------------------------------

    // raw / ui controller / midi reply -> midi out
    {
        let raw = raw.clone();
        let ui_controller = ui_controller.clone();
        let config = config.clone();
        let mut rx = raw.lock().unwrap().subscribe();
        let mut ui_rx = ui_controller.lock().unwrap().subscribe();
        let midi_out_tx = state.lock().unwrap().midi_out_tx.clone();
        tokio::spawn(async move {
            let make_cc = |idx: usize| -> Option<MidiMessage> {
                config.addr_to_cc_iter(idx)
                    .next()
                    .and_then(|cc| {
                        let value = raw.lock().unwrap().get(idx as usize);
                        value.map(|value| {
                            let control = config.cc_to_control(cc).unwrap().1;
                            let value = control.value_to_midi(value);
                            MidiMessage::ControlChange { channel: 1, control: cc, value }
                        })
                    })
            };
            let make_pc = || {
                let ui_controller = ui_controller.lock().unwrap();
                let v = ui_controller.get(&"program").unwrap();
                Some(MidiMessage::ProgramChange { channel: 1, program: v as u8 })
            };
            enum Program {
                EditBuffer,
                Current,
                All
            }
            let make_dump_request = |program: Program| {
                let ui_controller = ui_controller.lock().unwrap();
                let message = match program {
                    Program::EditBuffer => MidiMessage::ProgramEditBufferDumpRequest,
                    Program::Current => {
                        let current = ui_controller.get(&"program").unwrap();
                        MidiMessage::ProgramPatchDumpRequest { patch: current as u8 }
                    }
                    Program::All => MidiMessage::AllProgramsDumpRequest,
                };
                Some(message)
            };

            loop {
                let mut message: Option<MidiMessage> = None;
                let mut origin: u8 = UNSET;
                tokio::select! {
                  Ok(Event { key: idx, origin: o, .. }) = rx.recv() => {
                        message = make_cc(idx);
                        origin = o;
                    }
                  Ok(Event { key, origin: o, .. }) = ui_rx.recv() => {
                        message = match key.as_str() {
                            "program" => make_pc(),
                            "load_button" => make_dump_request(Program::EditBuffer),
                            "load_patch_button" => make_dump_request(Program::Current),
                            "load_all_button" => make_dump_request(Program::All),
                            _ => None
                        };
                        origin = o;
                    }
                }
                if origin == MIDI || message.is_none() {
                    continue;
                }
                match message {
                    Some(MidiMessage::ProgramChange { program, .. }) => {
                        program_change( raw.clone(), program, GUI);
                    }
                    _ => {}
                }
                match midi_out_tx.send(message.unwrap()) {
                    Ok(_) => {}
                    Err(err) => { error!("MIDI OUT error: {}", err); }
                }
            }
        });
    }

    // midi in -> raw / ui controller / midi out
    {
        let raw = raw.clone();
        let controller = controller.clone();
        let ui_controller = ui_controller.clone();
        let config = config.clone();
        let midi_out_tx = state.lock().unwrap().midi_out_tx.clone();
        let program_names = program_names.clone();
        tokio::spawn(async move {
            loop {
                let msg = midi_in_rx.recv().await;
                if msg.is_none() {
                    return; // shutdown
                }
                let msg = msg.unwrap();
                trace!(">> {:?}", msg);
                match msg {
                    MidiMessage::ControlChange { channel: _, control, value } => {
                        let controller = controller.lock().unwrap();
                        let mut raw = raw.lock().unwrap();

                        let addr = config.cc_to_addr(control);
                        if addr.is_none() {
                            warn!("Control for CC={} not defined!", control);
                            continue;
                        }
                        let control = config.cc_to_control(control).unwrap().1;
                        let value = control.value_from_midi(value);

                        raw.set(addr.unwrap() as usize, value, MIDI);
                    },
                    MidiMessage::ProgramChange { channel: _, program } => {
                        let mut ui_controller = ui_controller.lock().unwrap();
                        ui_controller.set("program", program as u16, MIDI);
                        program_change(raw.clone(), program, MIDI);
                    }
                    MidiMessage::ProgramEditBufferDump { ver, data } => {
                        let mut raw = raw.lock().unwrap();
                        let mut program_names = program_names.lock().unwrap();
                        if ver != 0 {
                            error!("Program dump version not supported: {}", ver);
                            continue;
                        }
                        if data.len() != config.program_size {
                            error!("Program size mismatch: expected {}, got {}",
                                  config.program_size, data.len());
                            continue;
                        }
                        program::load_patch_dump(
                            &mut raw, &mut program_names,
                            None, data.as_slice(), MIDI);
                    },
                    MidiMessage::ProgramEditBufferDumpRequest => {
                        let mut raw = raw.lock().unwrap();
                        let mut program_names = program_names.lock().unwrap();
                        let res = MidiMessage::ProgramEditBufferDump {
                            ver: 0,
                            data: program::store_patch_dump(&mut raw, &mut program_names, None, &config) };
                        midi_out_tx.send(res);
                    },
                    MidiMessage::ProgramPatchDump { patch, ver, data } => {
                        let mut raw = raw.lock().unwrap();
                        let mut program_names = program_names.lock().unwrap();
                        if ver != 0 {
                            error!("Program dump version not supported: {}", ver);
                            continue;
                        }
                        if data.len() != config.program_size {
                            error!("Program size mismatch: expected {}, got {}",
                                  config.program_size, data.len());
                            continue;
                        }
                        program::load_patch_dump(
                            &mut raw, &mut program_names,
                            Some(patch as usize), data.as_slice(), MIDI);
                    },
                    MidiMessage::ProgramPatchDumpRequest { patch } => {
                        let mut raw = raw.lock().unwrap();
                        let program_names = program_names.lock().unwrap();
                        let res = MidiMessage::ProgramPatchDump {
                            patch,
                            ver: 0,
                            data: program::store_patch_dump(&mut raw, &program_names,
                                                            Some(patch as usize), &config) };
                        midi_out_tx.send(res);
                    },
                    MidiMessage::AllProgramsDump { ver, data } => {
                        let mut raw = raw.lock().unwrap();
                        let mut program_names = program_names.lock().unwrap();
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
                            &mut raw, &mut program_names,
                            data.as_slice(), &config, MIDI);
                    },
                    MidiMessage::AllProgramsDumpRequest => {
                        let mut raw = raw.lock().unwrap();
                        let program_names = program_names.lock().unwrap();
                        let res = MidiMessage::AllProgramsDump {
                            ver: 0,
                            data: program::store_all_dump(&mut raw, &program_names, &config) };
                        midi_out_tx.send(res);
                    },

                    // pretend we're a POD
                    MidiMessage::UniversalDeviceInquiry { channel } => {
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

    // raw -----------------------------------------------------

    // raw -> controller
    {
        let raw = raw.clone();
        let controller = controller.clone();
        let config = config.clone();
        let mut rx = raw.lock().unwrap().subscribe();
        tokio::spawn(async move {
            loop {
                let mut addr: usize = usize::MAX-1;
                let mut origin: u8 = UNSET;
                match rx.recv().await {
                    Ok(Event { key: idx, origin: o, .. }) => {
                        addr = idx;
                        origin = o;
                    }
                    Err(e) => {
                        error!("Error in 'raw -> controller' rx: {}", e)
                    }
                }

                // discard messages originating from the 'controller -> raw' setting values
                // and anything beyond program name address start
                if origin == GUI || addr >= config.program_name_addr {
                    continue;
                }

                let mut controller = controller.lock().unwrap();
                let raw = raw.lock().unwrap();
                let mut control_configs = config.addr_to_control_iter(addr).peekable();
                if control_configs.peek().is_none() {
                    warn!("Control for address {} not found!", addr);
                    continue;
                }

                control_configs.for_each(|(name, config)| {
                    let value = raw.get(addr).unwrap() as u16;
                    let (caddr, bytes) = config.get_addr().unwrap();
                    match bytes {
                        1 => {
                            controller.set(name, value, MIDI);
                        }
                        2 => {
                            let bits = match config {
                                Control::RangeControl(RangeControl{ config: RangeConfig::Long { bits }, .. }) => bits,
                                _ => &[0u8, 0u8]
                            };
                            let cvalue = controller.get(name).unwrap();

                            let (mask, shift) = if addr == caddr as usize {
                                // first byte
                                let shift = bits[1];
                                let mask = ((1 << bits[0]) - 1) << shift;
                                (mask, shift)
                            } else {
                                let mask = (1 << bits[1]) - 1;
                                (mask, 0)
                            };
                            let v = (cvalue & !mask) | (value << shift);
                            controller.set(name, v, MIDI);

                        }
                        n => error!("Unsupported control size in bytes: {}", n)
                    }
                });
            }
        });

    }

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
                let (addr, bytes) = control_config.get_addr().unwrap();
                let modified = match bytes {
                    1 => {
                        raw.set_full(addr as usize, value as u8, GUI, signal);
                        true
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
                        raw.set_full((addr+1) as usize, b2 as u8, GUI, Signal::Force);
                        raw.set_full(addr as usize, b1 as u8, GUI, Signal::Force);
                        true
                    }
                    n => {
                        error!("Unsupported control size in bytes: {}", n);
                        false
                    }
                };
                if modified {
                    state.lock().unwrap().ui_event_tx.send(UIEvent::Modified(raw.page));
                }
            }
        });
    }


    // ---------------------------------------------------------

    // controller -> gui
    {
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
        let mut names_rx = program_names.lock().unwrap().subscribe();

        let transfer_up_sem = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let transfer_down_sem = Arc::new(std::sync::atomic::AtomicI32::new(0));

        glib::idle_add_local(move || {
            match rx.try_recv() {
                Ok(Event { key: name, .. }) => {
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
                    let vec = callbacks.get_vec(&name);
                    match vec {
                        None => { warn!("No GUI callback for '{}'", &name); },
                        Some(vec) => for cb in vec {
                            cb()
                        }
                    }
                    animate(&objects, &name, ui_controller.get(&name).unwrap());
                },
                _ => {}
            }
            match ui_event_rx.try_recv() {
                Ok(event) => {
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
                        }
                        UIEvent::Modified(page) => {
                            // patch index is 1-based
                            program_buttons.get_mut(page + 1)
                                .map(|button| button.set_modified(true));
                        }
                    }

                }
                _ => {}
            }
            match names_rx.try_recv() {
                Ok(Event { key: idx, .. }) => {
                    // patch index is 1-based
                    program_buttons.get(idx + 1).map(|button| {
                        let name = program_names.lock().unwrap().get(idx).unwrap_or_default();
                        button.set_name_label(&name);
                    });
                },
                _ => {}
            }

            Continue(true)
        });

    }

    // show the window and do init stuff...
    window.show_all();
    window.resize(1, 1);

    init_all(&config, controller.clone(), &objects);
    module.init(controller, raw)?;

    debug!("starting gtk main loop");
    gtk::main();

    Ok(())
}
