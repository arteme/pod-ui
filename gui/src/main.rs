extern crate gtk;

mod opts;
mod object_list;

use anyhow::*;
use gtk::prelude::*;
use glib::{Object, spawn_command_line_async};
use pod_core::pod::{MidiIn, MidiOut, PodConfigs};
use pod_core::controller::{Controller, GetSet};
use pod_core::program;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{Config, Control, AbstractControl, Format, EffectEntry};
use pod_core::config::{GUI, MIDI};
use std::collections::HashMap;
use crate::opts::*;
use pod_core::midi::MidiMessage;
use tokio::sync::broadcast::RecvError;
use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use crate::object_list::ObjectList;
use std::iter::repeat;
use tokio::sync::mpsc;
use core::time;
use std::thread;
use multimap::MultiMap;

fn clamp(v: f64) -> u16 {
    if v.is_nan() { 0 } else {
        if v.is_sign_negative() { 0 } else {
            if v > 0xffff as f64 { 0xffff } else { v as u16 }
        }
    }
}

type Callbacks = MultiMap<String, Box<dyn Fn() -> ()>>;

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
            controller.set(&name, v as u16, GUI);
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
    // controller -> gui
    {
        let objs = objs.clone();
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

                // to have these animate calls after the callback animate call we
                // schedule a one-off idle loop function
                let objs = objs.clone();
                gtk::idle_add(move || {
                    animate(&objs, "presence", presence as u16);
                    animate(&objs, "brightness_switch", bright_switch as u16);
                    Continue(false)
                });
            })
        )
    };
    Ok(())
}

fn effect_entry_for_value(config: &Config, value: u16) -> Option<(&EffectEntry, bool, usize)> {
    config.effects.iter()
        .enumerate()
        .flat_map(|(idx, effect)| {
            let delay = effect.delay.as_ref()
                .filter(|e| (value == e.id as u16))
                .map(|e| (e, true, idx));
            let clean = effect.clean.as_ref()
                .filter(|e| (value == e.id as u16))
                .map(|e| (e, false, idx));
            delay.or(clean)
        })
        .next()
        .or_else(|| {
            warn!("Effect select mapping for value {} not found!", value);
            None
        })

}

fn effect_select_from_midi(controller: &mut Controller, value: u16) -> Option<EffectEntry> {

    let entry_opt = effect_entry_for_value(&controller.config, value);
    if entry_opt.is_none() {
        return None;
    }
    let (entry, delay, index) = entry_opt.unwrap();
    let mut entry = entry.clone();

    controller.set("delay_enable", delay as u16, MIDI);
    // TODO: effect tweak, fx enable

    Some(entry)
}

fn effect_select_from_gui(controller: &mut Controller, value: u16) -> Option<EffectEntry> {

    let entry_opt = effect_entry_for_value(&controller.config, value);
    if entry_opt.is_none() {
        return None;
    }
    let (entry, delay, index) = entry_opt.unwrap();
    let mut entry = entry.clone();
    let mut delay_enable = controller.get("delay_enable").unwrap() != 0;

    if delay && !delay_enable {
        // when effect select from UI selects an effect with delay, turn on delay_enabled
        controller.set("delay_enable", delay as u16, MIDI);
        delay_enable = delay;
    }

    if delay_enable {
        // if delay_enabled is set, send the correct effect_select value with delay
        entry = controller.config.effects.get(index).and_then(|e| e.delay.as_ref()).unwrap().clone();
        controller.set("effect_select", entry.id as u16, GUI);
    }
    // TODO: effect tweak, fx enable
    Some(entry)
}

fn wire_effect_select(controller: Arc<Mutex<Controller>>, callbacks: &mut Callbacks) -> Result<()> {

    // effect_select -> delay_enable
    {
        let controller = controller.clone();
        let name = "effect_select".to_string();
        callbacks.insert(
            name.clone(),
            Box::new(move || {
                let mut controller = controller.lock().unwrap();
                let (v, origin) = controller.get_origin(&name).unwrap();

                let entry = match origin {
                    MIDI => effect_select_from_midi(&mut controller, v),
                    GUI => effect_select_from_gui(&mut controller, v),
                    _ => None
                };
            })
        )
    }

    // delay_enable -> effect_select
    {
        let controller = controller.clone();
        let name = "delay_enable".to_string();
        callbacks.insert(
            name.clone(),
            Box::new(move || {
                let mut controller = controller.lock().unwrap();
                let (v, origin) = controller.get_origin(&name).unwrap();
                let effect_select = controller.get("effect_select").unwrap();

                if v != 0 && origin == GUI {
                    let (_, delay, idx) =
                        effect_entry_for_value(&controller.config, effect_select).unwrap();
                    if !delay {
                        // if `delay_enable` was switched on in the UI and if coming from
                        // an effect which didn't have delay to begin with, check if it can
                        // have a delay at all (POD 2.0 rotary cannot). If not, then switch
                        // to plain "delay" effect.
                        let need_reset = controller.config.effects.get(idx)
                            .map(|e| e.delay.is_none()).unwrap_or(false);
                        if need_reset {
                            let v = controller.config.effects[0].delay.as_ref().unwrap().id;
                            controller.set("effect_select", v as u16, GUI);
                        }
                    }
                }
            })
        )
    }

    Ok(())
}

fn init_combo<T, F>(controller: &Controller, objs: &ObjectList,
              name: &str, list: &Vec<T>, get_name: F) -> Result<()>
    where F: Fn(&T) -> &str
{
    let select = objs.ref_by_name::<gtk::ComboBoxText>(name)?;
    for item in list.iter() {
        let name = get_name(item);
        select.append_text(name);
    }

    let v = controller.get(name).unwrap();
    select.set_active(Some(v as u32));

    Ok(())
}

fn animate(objs: &ObjectList, control_name: &str, control_value: u16) {
    let prefix1 = format!("{}=", control_name);
    let prefix2 = format!("{}:", control_value);
    let catchall = "*:";
    let prefix_len = prefix1.len() + prefix2.len();
    let catchall_len = prefix1.len() + catchall.len();
    debug!("Animate: {:?}?", control_name);
    objs.widgets_by_class_match(&|class_name| class_name.starts_with(prefix1.as_str()))
        .flat_map(|(widget, classes)| {
            let get_classes = |suffix: &str| {
                let full_len = prefix1.len() + suffix.len();
                classes.iter()
                    .filter(|c| &c[prefix1.len()..full_len] == suffix)
                    .map(|c| c[full_len..].to_string()).collect::<Vec<_>>()
            };

            let mut c = get_classes(&prefix2);
            if c.is_empty() {
                c = get_classes(catchall);
            }
            debug!("Animate: {:?} for {:?}", c, widget);

            repeat(widget.clone()).zip(c)
        })
        .for_each(|(widget, cls)| {
            match cls.as_str() {
                "show" => widget.show(),
                "hide" => widget.hide(),
                "opacity=0" => widget.set_opacity(0f64),
                "opacity=1" => widget.set_opacity(1f64),
                "enable" => widget.set_sensitive(true),
                "disable" => widget.set_sensitive(false),
                _ => {
                    warn!("Unknown animation command {:?} for widget {:?}", cls, widget)
                }
            }
        });
}


fn wire_all(controller: Arc<Mutex<Controller>>, objs: &ObjectList) -> Result<Callbacks> {
    let mut callbacks = Callbacks::new();

    objs.named_objects()
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

                            match &c.format {
                                Format::Callback(f) => {
                                    let c = c.clone();
                                    let f = f.clone();
                                    scale.connect_format_value(move |_, val| f(&c, val));
                                },
                                Format::Data(data) => {
                                    let data = data.clone();
                                    scale.connect_format_value(move |_, val| data.format(val));
                                },
                                Format::Labels(labels) => {
                                    let labels = labels.clone();
                                    scale.connect_format_value(move |_, val| labels.get(val as usize).unwrap_or(&"".into()).clone());

                                }
                                Format::None | _ => {}
                            }
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
                        controller.set(&name, adj.get_value() as u16, GUI);
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
                        controller.set(&name, check.get_active() as u16, GUI);
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
                        controller.set(&name, value.unwrap(), GUI);
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
                let cfg = match controller.get_config(&name) {
                    Some(Control::Select(c)) => Some(c),
                    _ => {
                        warn!("Control {:?} is not a select control!", name);
                        None
                    }
                };
                let mut signal_id;

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let to_midi = cfg.clone().and_then(|select| select.to_midi);
                    let name = name.clone();
                    signal_id = combo.connect_changed(move |combo| {
                        combo.get_active().map(|v| {
                            let v1 = to_midi.as_ref()
                                .and_then(|vec| vec.get(v as usize))
                                .or_else(|| {
                                    warn!("To midi conversion failed for select {:?} value {}",
                                    name, v);
                                    None
                                })
                                .unwrap_or(&v);

                            controller.set(&name, *v1 as u16, GUI);
                        });
                    });
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let from_midi = cfg.and_then(|select| select.from_midi);
                    let name = name.clone();
                    let combo = combo.clone();
                    callbacks.insert(
                        name.clone(),
                        Box::new(move || {
                            let v = controller.get(&name).unwrap() as u32;
                            let v1 = from_midi.as_ref()
                                .and_then(|vec| vec.get(v as usize))
                                .or_else(|| {
                                    warn!("From midi conversion failed for select {:?} value {}",
                                    name, v);
                                    None
                                })
                                .unwrap_or(&v);
                            // TODO: signal_handler_block is a hack because actual value set
                            //       to the UI control is not the same as what came from MIDI,
                            //       so as to not override the MIDI-set value, block the "changed"
                            //       signal handling altogether
                            glib::signal::signal_handler_block(&combo, &signal_id);
                            combo.set_active(Some(*v1));
                            glib::signal::signal_handler_unblock(&combo, &signal_id);
                        })
                    )
                }
            });
        });

    wire_vol_pedal_position(controller.clone(), objs, callbacks.borrow_mut())?;
    wire_amp_select(controller.clone(), objs, callbacks.borrow_mut())?;
    wire_effect_select(controller.clone(), callbacks.borrow_mut())?;

    Ok(callbacks)
}


fn init_all(config: &Config, controller: Arc<Mutex<Controller>>, objs: &ObjectList) -> () {
    for name in &config.init_controls {
        animate(objs, &name, controller.get(&name).unwrap());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init()?;

    let opts: Opts = Opts::parse();
    let mut midi_in = MidiIn::new_for_address(opts.input)
        .context("Failed to initialize MIDI").unwrap();
    let mut midi_out = MidiOut::new_for_address(opts.output)
        .context("Failed to initialize MIDI").unwrap();
    let (midi_tx, mut midi_rx) = mpsc::unbounded_channel();

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let configs = PodConfigs::new()?;
    let config: Config = configs.by_name(&"POD 2.0".into()).context("Config not found by name 'POD 2.0'")?;
    let controller = Arc::new(Mutex::new(Controller::new(config.clone())));

    let builder = gtk::Builder::new_from_file("src/pod.glade");
    let objects = ObjectList::new(&builder);
    objects.dump_debug();

    init_combo(controller.lock().unwrap().deref(), &objects,
               "cab_select", &config.cab_models, |s| s.as_str() )?;
    init_combo(controller.lock().unwrap().deref(), &objects,
               "amp_select", &config.amp_models, |amp| amp.name.as_str() )?;
    init_combo(controller.lock().unwrap().deref(), &objects,
               "effect_select", &config.effects, |eff| eff.name.as_str() )?;

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

            fn handle_cc(name: &str, controller: &Controller) -> Option<MidiMessage> {
                let (config, val, origin) = {
                    let config = controller.get_config(name).unwrap();
                    let (val, origin) = controller.get_origin(name).unwrap();
                    (config.clone(), val, origin)
                };
                if origin != GUI {
                    return None;
                }

                let scale= match &config {
                    Control::SwitchControl(_) => 64u16,
                    Control::RangeControl(c) => 127 / c.to as u16,
                    _ => 1
                };
                let msg = MidiMessage::ControlChange { channel: 1, control: config.get_cc().unwrap(), value: (val * scale) as u8 };
                Some(msg)
            }

            loop {
                let message: Option<MidiMessage>;
                tokio::select! {
                  Some(msg) = midi_rx.recv() => {
                        message = Some(msg);
                    },
                  Ok(name) = rx.recv() => {
                        message = handle_cc(name.as_str(), &controller.lock().unwrap());
                    },
                }
                if message.is_none() {
                    continue;
                }
                match midi_out.send(&message.unwrap().to_bytes()) {
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
                        let config = controller.get_config_by_cc(control);
                        if config.is_none() {
                            warn!("Control for CC={} not defined!", control);
                            continue;
                        }
                        let (name, config) = config.unwrap();
                        let name = name.clone();
                        let scale= match &config {
                            Control::SwitchControl(_) => 64u16,
                            Control::RangeControl(c) => 127 / c.to as u16,
                            _ => 1
                        };
                        controller.set(&name, value as u16 / scale, MIDI);
                    },
                    MidiMessage::ProgramEditBufferDump { ver, data } => {
                        let mut controller = controller.lock().unwrap();
                        if data.len() != controller.config.program_size {
                            warn!("Program size mismatch: expected {}, got {}",
                                  controller.config.program_size, data.len());
                        }
                        program::load_dump(controller.deref_mut(), data.as_slice(), MIDI);
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
                            ver: String::from("0223")
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

    // controller -> gui
    {
        let controller = controller.clone();
        let objects = objects.clone();

        let mut rx = {
            let controller = controller.lock().unwrap();
            controller.subscribe()
        };
        gtk::idle_add(move || {
            match rx.try_recv() {
                Ok(name) => {
                    let vec = callbacks.get_vec(&name);
                    match vec {
                        None => { warn!("No GUI callback for '{}'", name); },
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

            Continue(true)
        });

    }

    // show the window and do init stuff...
    window.show_all();
    init_all(&config, controller.clone(), &objects);

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
