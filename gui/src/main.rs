extern crate gtk;

mod opts;
mod object_list;

use anyhow::*;
use gtk::prelude::*;
use glib::Object;
use pod_core::pod::{MidiIn, MidiOut, PodConfigs};
use pod_core::controller::{Controller, GetSet};
use pod_core::program;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{Config, Control, AbstractControl};
use std::collections::HashMap;
use crate::opts::*;
use pod_core::midi::MidiMessage;
use tokio::sync::broadcast::RecvError;
use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use crate::object_list::ObjectList;
use tokio::sync::mpsc;
use core::time;
use std::thread;

fn clamp(v: f64) -> u16 {
    if v.is_nan() { 0 } else {
        if v.is_sign_negative() { 0 } else {
            if v > 0xffff as f64 { 0xffff } else { v as u16 }
        }
    }
}

type Callbacks = HashMap<String, Box<dyn Fn() -> ()>>;

fn wire_vol_pedal_position(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let name = "vol_pedal_position".to_string();
    let vol_pedal_position = objs.ref_by_name::<gtk::Button>(&name)?;
    let amp_enable = objs.ref_by_name::<gtk::Widget>("amp_enable")?;
    let volume_enable = objs.ref_by_name::<gtk::Widget>("volume_enable")?;

    let set_in_order = {
        let vol_pedal_position = vol_pedal_position.clone();

        move |volume_post_amp: bool| {
            let ancestor = amp_enable.get_ancestor(gtk::Grid::static_type()).unwrap();
            let grid = ancestor.dynamic_cast_ref::<gtk::Grid>().unwrap();
            grid.remove(&amp_enable);
            grid.remove(&volume_enable);

            let (volume_left, amp_left) = match volume_post_amp {
                false => {
                    vol_pedal_position.set_label(">");
                    (1, 2)
                },
                true => {
                    vol_pedal_position.set_label("<");
                    (2, 1)
                }
            };
            grid.attach(&amp_enable, amp_left, 1, 1, 1);
            grid.attach(&volume_enable, volume_left, 1, 1, 1);
        }
    };

    set_in_order(false);

    // gui -> controller
    {
        let controller = controller.clone();
        let name = name.clone();
        vol_pedal_position.connect_clicked(move |_| {
            let mut controller = controller.lock().unwrap();
            let v = controller.get(&name).unwrap() > 0;
            let v = !v; // toggling
            controller.set(&name, v as u16);
        });
    }

    // controller -> gui
    {
        let controller = controller.clone();
        callbacks.insert(
            name.clone(),
            Box::new(move || {
                let v = {
                    let controller = controller.lock().unwrap();
                    controller.get(&name).unwrap()
                };
                set_in_order(v > 0);
            })
        )
    };
    Ok(())
}

fn wire_amp_select(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let presence_widget = objs.ref_by_name::<gtk::Widget>("presence")?;
    let presence_label_widget = objs.ref_by_name::<gtk::Label>("presence_label")?;

    // controller -> gui
    {
        let controller = controller.clone();
        let name = "amp_select".to_string();
        callbacks.insert(
            name.clone(),
            Box::new(move || {
                let (presence, bright_switch) = {
                    let controller = controller.lock().unwrap();
                    let v = controller.get(&name).unwrap();
                    let amp = controller.config.amp_models.get(v as usize).unwrap();
                    (amp.presence, amp.bright_switch)
                };

                presence_widget.set_visible(presence);
                // If I hide all widgets in the column, the others will spread out. Instead
                // I set presence label opacity to 0.
                //presence_label_widget.set_visible(presence);
                presence_label_widget.set_opacity(presence as i8 as f64 * 1.0);
            })
        )
    };
    Ok(())
}

fn init_cab_select(config: &Config, controller: &Controller, objs: &ObjectList) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBoxText>("cab_select")?;
    for name in config.cab_models.iter() {
        select.append_text(name.as_str());
    }

    let v = controller.get("cab_select").unwrap();
    select.set_active(Some(v as u32));

    Ok(())
}

fn init_amp_select(config: &Config, controller: &Controller, objs: &ObjectList) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBoxText>("amp_select")?;
    for amp in config.amp_models.iter() {
        select.append_text(amp.name.as_str());
    }

    let v = controller.get("amp_select").unwrap();
    select.set_active(Some(v as u32));

    Ok(())
}


fn wire_all(controller: Arc<Mutex<Controller>>, objs: &ObjectList) -> Result<Callbacks> {
    let mut callbacks = Callbacks::new();

    objs.obj_iter()
        .for_each(|(obj, name)| {
            {
                let controller = controller.lock().unwrap();
                if !controller.has(&name) {
                    warn!("Not wiring {:?}", name);
                    return;
                }
            }

            info!("Wiring {:?} {:?}", name, obj);
            obj.dynamic_cast_ref::<gtk::Scale>().map(|scale| {
                // wire GtkScale and its internal GtkAdjustment
                let adj = scale.get_adjustment();
                info!("adj {:?}", adj);
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::RangeControl(c)) => {
                            adj.set_lower(c.from as f64);
                            adj.set_upper(c.to as f64);
                        },
                        _ => {
                            warn!("Control {:?} is not a range control!", name)
                        }
                    }
                }

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    adj.connect_value_changed(move |adj| {
                        let mut controller = controller.lock().unwrap();
                        controller.set(&name, adj.get_value() as u16);
                    });
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    callbacks.insert(
                        name.clone(),
                        Box::new(move || {
                            // TODO: would be easier if value is passed in the message and
                            //       into this function without the need to look it up from the controller
                            let v = {
                                let controller = controller.lock().unwrap();
                                controller.get(&name).unwrap()
                            };
                            adj.set_value(v as f64);
                        })
                    )
                }
            });
            obj.dynamic_cast_ref::<gtk::CheckButton>().map(|check| {
                // wire GtkCheckBox
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::SwitchControl(_)) => {},
                        _ => {
                            warn!("Control {:?} is not a switch control!", name)
                        }
                    }
                }

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    check.connect_toggled(move |check| {
                        controller.set(&name, check.get_active() as u16);
                    });
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    let check = check.clone();
                    callbacks.insert(
                        name.clone(),
                        Box::new(move || {
                            let v = controller.get(&name).unwrap();
                            check.set_active(v > 0);
                        })
                    )
                }
            });
            obj.dynamic_cast_ref::<gtk::RadioButton>().map(|radio| {
                // wire GtkRadioButton
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::SwitchControl(_)) => {},
                        _ => {
                            warn!("Control {:?} is not a switch control!", name)
                        }
                    }
                }

                // this is a group, look up the children
                let group = radio.get_group();

                // wire gui -> controller
                for radio in group.clone() {
                    let controller = controller.clone();
                    let name = name.clone();
                    let radio_name = ObjectList::object_name(&radio).unwrap();
                    let value = radio_name.find(':')
                        .map(|pos| &radio_name[pos+1..]).map(|str| str.parse::<u16>().unwrap());
                    if value.is_none() {
                        // value not of "name:N" pattern, skip
                        continue;
                    }
                    radio.connect_toggled(move |radio| {
                        if !radio.get_active() { return; }
                        let mut controller = controller.lock().unwrap();
                        controller.set(&name, value.unwrap());
                    });
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    callbacks.insert(
                        name.clone(),
                        Box::new(move || {
                            let v = {
                                let controller = controller.lock().unwrap();
                                controller.get(&name).unwrap()
                            };
                            let item_name = format!("{}:{}", name, v);
                            group.iter().find(|radio| ObjectList::object_name(*radio).unwrap_or_default() == item_name)
                                .and_then(|item| {
                                    item.set_active(true);
                                    Some(())
                                })
                                .or_else( || {
                                    error!("GtkRadioButton not found with name '{}'", name);
                                    None
                                });
                        })
                    )
                }
            });
            obj.dynamic_cast_ref::<gtk::ComboBoxText>().map(|combo| {
                // wire GtkComboBox
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::Select(_)) => {},
                        _ => {
                            warn!("Control {:?} is not a select control!", name)
                        }
                    }
                }

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    combo.connect_changed(move |combo| {
                        combo.get_active().map(|v| {
                            controller.set(&name, v as u16);
                        });
                    });
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    let combo = combo.clone();
                    callbacks.insert(
                        name.clone(),
                        Box::new(move || {
                            let v = controller.get(&name).unwrap();
                            combo.set_active(Some(v as u32));
                        })
                    )
                }
            });
        });

    wire_vol_pedal_position(controller.clone(), objs, callbacks.borrow_mut())?;
    wire_amp_select(controller, objs, callbacks.borrow_mut())?;

    Ok(callbacks)
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init()?;

    let opts: Opts = Opts::parse();
    let mut midi_in = MidiIn::new(opts.input)
        .context("Failed to initialize MIDI").unwrap();
    let mut midi_out = MidiOut::new(opts.output)
        .context("Failed to initialize MIDI").unwrap();
    let (midi_tx, mut midi_rx) = mpsc::unbounded_channel();

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let configs = PodConfigs::new()?;
    let config: Config = configs.by_name(&"POD 2.0".into()).context("Config not found by name 'POD 2.0'")?;
    let controller = Arc::new(Mutex::new(Controller::new(config.clone())));

    let builder = gtk::Builder::new_from_file("src/pod.glade");
    let objects = ObjectList::new(&builder);

    init_cab_select(&config, controller.lock().unwrap().deref(), &objects)?;
    init_amp_select(&config, controller.lock().unwrap().deref(), &objects)?;

    let callbacks = wire_all(controller.clone(), &objects)?;

    let window: gtk::Window = builder.get_object("app_win").unwrap();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // midi ----------------------------------------------------

    // controller / midi in -> midi out
    {
        let controller = controller.clone();
        let mut rx = {
            let controller = controller.lock().unwrap();
            controller.subscribe()
        };
        tokio::spawn(async move {

            fn handle_cc(name: &str, controller: &Controller) -> MidiMessage {
                let (config, val) = {
                    let config = controller.get_config(&name).unwrap();
                    let val = controller.get(&name).unwrap();
                    (config.clone(), val)
                };
                let scale= match &config {
                    Control::SwitchControl(_) => 64u16,
                    Control::RangeControl(c) => 127 / c.to as u16,
                    _ => 1
                };
                MidiMessage::ControlChange { channel: 1, control: config.get_cc().unwrap(), value: (val * scale) as u8 }
            }

            loop {
                let message: MidiMessage;
                tokio::select! {
                  Some(msg) = midi_rx.recv() => { message = msg },
                  Ok(name) = rx.recv() => { message = handle_cc(name.as_str(), &controller.lock().unwrap()) },
                }
                match midi_out.send(&message.to_bytes()) {
                    Ok(_) => {}
                    Err(err) => { error!("MIDI OUT error: {}", err); }
                }
            }
        });
    }

    // midi in -> controller / midi out
    {
        let controller = controller.clone();
        let config = config.clone();
        tokio::spawn(async move {
            loop {
                let data = midi_in.recv().await;
                if data.is_none() {
                    return; // shutdown
                }
                let event = MidiMessage::from_bytes(data.unwrap());
                let msg: MidiMessage = match event {
                    Ok(msg) =>  msg,
                    Err(err) => { error!("Error parsing MIDI message: {:?}", err); continue }
                };
                match msg {
                    MidiMessage::ControlChange { channel: _, control, value } => {
                        let mut controller = controller.lock().unwrap();
                        let (name, config) = controller.get_config_by_cc(control).unwrap();
                        let name = name.clone();
                        let scale= match &config {
                            Control::SwitchControl(_) => 64u16,
                            Control::RangeControl(c) => 127 / c.to as u16,
                            _ => 1
                        };
                        controller.set(&name, value as u16 / scale);
                    },
                    MidiMessage::ProgramEditBufferDump { ver, data } => {
                        let mut controller = controller.lock().unwrap();
                        if data.len() != controller.config.program_size {
                            warn!("Program size mismatch: expected {}, got {}",
                                  controller.config.program_size, data.len());
                        }
                        program::load_dump(controller.deref_mut(), data.as_slice());
                    },
                    MidiMessage::ProgramEditBufferDumpRequest => {
                        let controller = controller.lock().unwrap();
                        let res = MidiMessage::ProgramEditBufferDump {
                            ver: 0,
                            data: program::dump(&controller) };
                        midi_tx.send(res);
                    },
                    MidiMessage::ProgramPatchDumpRequest { patch } => {
                        // TODO: For now answer with the contents of the edit buffer to any patch
                        //       request
                        let controller = controller.lock().unwrap();
                        let res = MidiMessage::ProgramPatchDump {
                            patch,
                            ver: 0,
                            data: program::dump(&controller) };
                        midi_tx.send(res);
                    },

                    // pretend we're a POD
                    MidiMessage::UniversalDeviceInquiry { channel } => {

                        let res = MidiMessage::UniversalDeviceInquiryResponse {
                            channel,
                            family: config.family,
                            member: config.member,
                            ver: String::from("0200")
                        };
                        midi_tx.send(res);
                    }

                    _ => {
                        warn!("Unhandled MIDI message: {:?}", msg);
                    }
                }
            }
        });
    }
    // ---------------------------------------------------------

    window.show_all();
    let mut rx = {
        let controller = controller.lock().unwrap();
        controller.subscribe()
    };
    gtk::idle_add(move || {
        match rx.try_recv() {
            Ok(name) => {
                let cb = callbacks.get(&name);
                match cb {
                    None => { warn!("No GUI callback for '{}'", name); },
                    Some(cb) => cb(),
                }
            Err(_) => {
                thread::sleep(time::Duration::from_millis(100));
            },
        }

        Continue(true)
    });

    debug!("starting gtk main loop");
    gtk::main();

    /*
    loop {
        gtk::main_iteration_do(false);
        sleep_ms(1);
    }
     */

    Ok(())
}
