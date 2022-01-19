mod opts;

use anyhow::*;

use pod_gtk::*;
use pod_gtk::gtk::prelude::*;
use pod_core::pod::{MidiIn, MidiOut};
use pod_core::controller::Controller;
use pod_core::program;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{AbstractControl, Config, Control, Select};
use pod_core::config::{GUI, MIDI, register_config, UNSET};
use crate::opts::*;
use pod_core::midi::MidiMessage;
use tokio::sync::mpsc;
use core::time;
use std::thread;
use pod_core::raw::Raw;
use pod_core::store::{Event, Signal, Store};
use core::result::Result::Ok;
use std::sync::atomic::Ordering;
use std::time::Duration;

fn init_all(config: &Config, controller: Arc<Mutex<Controller>>, objs: &ObjectList) -> () {
    for name in &config.init_controls {
        animate(objs, &name, controller.get(&name).unwrap());
    }
}

enum UIEvent {
    MidiTx,
    MidiRx
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init()?;

    let opts: Opts = Opts::parse();

    let (midi_in_tx, mut midi_in_rx) = mpsc::unbounded_channel::<MidiMessage>();
    let (midi_out_tx, mut midi_out_rx) = mpsc::unbounded_channel::<MidiMessage>();
    let (ui_event_tx, mut ui_event_rx) = mpsc::unbounded_channel::<UIEvent>();

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let module = pod_mod_pod2::module();
    let config = module.config();

    // register POD 2.0 module
    register_config(&config);

    // autodetect/open midi
    let autodetect = match (&opts.input, &opts.output) {
        (None, None) => true,
        _ => false
    };
    let (mut midi_in, mut midi_out) = if autodetect {
        pod_core::pod::autodetect().await?
    } else {
        let mut midi_in = MidiIn::new_for_address(opts.input)
            .context("Failed to initialize MIDI").unwrap();
        let midi_out = MidiOut::new_for_address(opts.output)
            .context("Failed to initialize MIDI").unwrap();

        (midi_in, midi_out)
    };

    let raw = Arc::new(Mutex::new(Raw::new(config.program_size)));

    let controller = Arc::new(Mutex::new(Controller::new(config.clone())));

    let ui = gtk::Builder::from_string(include_str!("ui.glade"));

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

    let pod_ui = module.widget();
    let app_grid: gtk::Box = ui.object("app_grid").unwrap();
    app_grid.add(&pod_ui);

    let mut callbacks = Callbacks::new();
    module.wire(controller.clone(), raw.clone(), &mut callbacks)?;

    let objects = ObjectList::new(&ui) + module.objects();

    // midi ----------------------------------------------------

    // midi in
    {
        let midi_in_tx = midi_in_tx.clone();
        let ui_event_tx = ui_event_tx.clone();

        tokio::spawn(async move {
            while let Some(bytes) = midi_in.recv().await {
                match MidiMessage::from_bytes(bytes) {
                    Ok(msg) => { midi_in_tx.send(msg); () },
                    Err(err) => error!("Error deserializing MIDI message: {}", err)
                }
                ui_event_tx.send(UIEvent::MidiRx);
            }
        });
    }

    // midi out
    {
        let ui_event_tx = ui_event_tx.clone();

        tokio::spawn(async move {
            while let Some(msg) = midi_out_rx.recv().await {
                let bytes = msg.to_bytes();
                midi_out.send(&bytes);
                ui_event_tx.send(UIEvent::MidiTx);
            }
        });
    }

    // raw / midi reply -> midi out
    {
        let raw = raw.clone();
        let config = config.clone();
        let mut rx = raw.lock().unwrap().subscribe();
        let midi_out_tx = midi_out_tx.clone();
        tokio::spawn(async move {
            let make_cc = |idx: usize| -> Option<MidiMessage> {
                config.addr_to_cc_iter(idx)
                    .next()
                    .and_then(|cc| {
                        let value = raw.lock().unwrap().get(idx as usize);
                        value.map(|v| {
                            MidiMessage::ControlChange { channel: 1, control: cc, value: v }
                        })
                    })
            };

            loop {
                let mut message: Option<MidiMessage> = None;
                let mut origin: u8 = UNSET;
                tokio::select! {
                  Ok(Event { key: idx, origin: o, .. }) = rx.recv() => {
                        message = make_cc(idx);
                        origin = o;
                    },
                }
                if origin == MIDI || message.is_none() {
                    continue;
                }
                match midi_out_tx.send(message.unwrap()) {
                    Ok(_) => {}
                    Err(err) => { error!("MIDI OUT error: {}", err); }
                }
            }
        });
    }

    // midi in -> raw / midi out
    {
        let raw = raw.clone();
        let controller = controller.clone();
        let config = config.clone();
        tokio::spawn(async move {
            loop {
                let msg = midi_in_rx.recv().await;
                if msg.is_none() {
                    return; // shutdown
                }
                let msg = msg.unwrap();
                /*
                let event = MidiMessage::from_bytes(data.unwrap());
                let msg: MidiMessage = match event {
                    Ok(msg) =>  msg,
                    Err(err) => { error!("Error parsing MIDI message: {:?}", err); continue }
                };

                 */
                match msg {
                    MidiMessage::ControlChange { channel: _, control, value } => {
                        let controller = controller.lock().unwrap();
                        let mut raw = raw.lock().unwrap();

                        let addr = config.cc_to_addr(control);
                        if addr.is_none() {
                            warn!("Control for CC={} not defined!", control);
                            continue;
                        }

                        raw.set(addr.unwrap() as usize, value, MIDI);
                    },
                    MidiMessage::ProgramEditBufferDump { ver, data } => {
                        let mut controller = controller.lock().unwrap();
                        if data.len() != controller.config.program_size {
                            warn!("Program size mismatch: expected {}, got {}",
                                  controller.config.program_size, data.len());
                        }
                        program::load_dump(&mut controller, data.as_slice(), MIDI);
                    },
                    MidiMessage::ProgramEditBufferDumpRequest => {
                        let controller = controller.lock().unwrap();
                        let res = MidiMessage::ProgramEditBufferDump {
                            ver: 0,
                            data: program::dump(&controller) };
                        midi_out_tx.send(res);
                    },
                    MidiMessage::ProgramPatchDumpRequest { patch } => {
                        // TODO: For now answer with the contents of the edit buffer to any patch
                        //       request
                        let controller = controller.lock().unwrap();
                        let res = MidiMessage::ProgramPatchDump {
                            patch,
                            ver: 0,
                            data: program::dump(&controller) };
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
                tokio::select! {
                Ok(Event { key: idx, origin: o, .. }) = rx.recv() => {
                        addr = idx;
                        origin = o;
                    }
                }

                if origin == GUI {
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
                    let scale= match &config {
                        Control::SwitchControl(_) => 64u16,
                        Control::RangeControl(c) => 127 / c.to as u16,
                        _ => 1
                    };
                    let value = raw.get(addr).unwrap() as u16 / scale;
                    let value = match config {
                        Control::Select(Select { from_midi: Some(from_midi), .. }) => {
                            from_midi.get(value as usize)
                                .or_else(|| {
                                    warn!("From midi conversion failed for select {:?} value {}",
                                name, value);
                                    None
                                })
                                .unwrap_or(&value)
                        },
                        _ => &value
                    };
                    controller.set(name, *value, MIDI);
                });
            }
        });

    }

    // controller -> raw
    {
        let raw = raw.clone();
        let controller = controller.clone();
        let config = config.clone();
        let mut rx = controller.lock().unwrap().subscribe();
        tokio::spawn(async move {
            loop {
                let mut name: String = String::new();
                let mut signal: Signal = Signal::None;
                tokio::select! {
                    Ok(Event { key: n, signal: s, .. }) = rx.recv() => {
                        name = n;
                        signal = s;
                    }
                }

                let controller = controller.lock().unwrap();
                let mut raw = raw.lock().unwrap();
                let control_config = controller.get_config(&name);
                if control_config.is_none() {
                    warn!("Control {:?} not found!", &name);
                    continue;
                }

                let (val, origin) = controller.get_origin(&name).unwrap();
                if origin != GUI {
                    continue;
                }

                let control_config = control_config.unwrap();
                let scale = match control_config {
                    Control::SwitchControl(_) => 64u16,
                    Control::RangeControl(c) => 127 / c.to as u16,
                    _ => 1
                };
                let value = val * scale;
                let value = match control_config {
                    Control::Select(Select { to_midi: Some(to_midi), .. }) => {
                        to_midi.get(value as usize)
                            .or_else(|| {
                                warn!("To midi conversion failed for select {:?} value {}",
                                name, value);
                                None
                            })
                            .unwrap_or(&value)
                    },
                    _ => &value
                };

                let addr = control_config.get_addr().unwrap().0; // TODO: multibyte!
                raw.set_full(addr as usize, *value as u8, GUI, signal);
            }
        });
    }


    // ---------------------------------------------------------

    // controller -> gui
    {
        let controller = controller.clone();
        let objects = objects.clone();

        let mut rx = {
            let controller = controller.lock().unwrap();
            controller.subscribe()
        };

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
                Err(_) => {
                    thread::sleep(time::Duration::from_millis(100));
                },
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
                    }

                }
                _ => {}
            }

            Continue(true)
        });

    }

    // show the window and do init stuff...
    window.show_all();

    window.resize(1, 1);



    init_all(&config, controller.clone(), &objects);

    debug!("starting gtk main loop");
    gtk::main();

    Ok(())
}
