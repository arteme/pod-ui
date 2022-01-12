extern crate gtk;

mod opts;
mod object_list;

use anyhow::*;
use gtk::prelude::*;
use pod_core::pod::{MidiIn, MidiOut, PodConfigs};
use pod_core::controller::{Controller, ControllerStoreExt};
use pod_core::program;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{Config, Control, AbstractControl, Format, EffectEntry, Select};
use pod_core::config::{GUI, MIDI, UNSET};
use crate::opts::*;
use pod_core::midi::MidiMessage;
use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use crate::object_list::ObjectList;
use std::iter::repeat;
use tokio::sync::mpsc;
use core::time;
use std::thread;
use multimap::MultiMap;
use pod_core::raw::Raw;
use pod_core::store::{Event, Signal, Store, StoreSetIm};
use core::result::Result::Ok;

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
            let ancestor = amp_enable.ancestor(gtk::Grid::static_type()).unwrap();
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
                glib::idle_add_local(move || {
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

    let value = controller.get("effect_select:raw").unwrap();
    let entry_opt = effect_entry_for_value(&controller.config, value);
    if entry_opt.is_none() {
        return None;
    }
    let (entry, delay, index) = entry_opt.unwrap();
    let entry = entry.clone();

    controller.set("delay_enable", delay as u16, MIDI);
    // TODO: effect tweak, fx enable

    Some(entry)
}

fn effect_select_from_gui(controller: &mut Controller, value: u16) -> Option<EffectEntry> {

    let effect = &controller.config.effects[value as usize];
    let delay_enable = controller.get("delay_enable").unwrap() != 0;

    let (delay, clean) = (effect.delay.as_ref(), effect.clean.as_ref());

    // if delay_enabled is set, try to set an effect with delay (fallback to clean),
    // otherwise try to set clean effect (fallback to effect with delay)
    let entry =
        (if delay_enable { delay.or(clean) } else { clean.or(delay) })
            .unwrap().clone();

    controller.set("effect_select:raw", entry.id as u16, GUI);
    // TODO: effect tweak, fx enable

    Some(entry)
}

fn effect_select_send_controls(controller: &mut Controller, effect: &EffectEntry) {
    for name in &effect.controls {
        controller.get(&name)
            .and_then(|v| {
                controller.borrow_mut()
                    .set_full(name, v, GUI, Signal::Force);
                Some(())
            });
    }
}

fn wire_effect_select(controller: Arc<Mutex<Controller>>, raw: Arc<Mutex<Raw>>, callbacks: &mut Callbacks) -> Result<()> {

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
                    GUI => {
                        effect_select_from_gui(&mut controller, v);
                        // HACK: adjust UI to the "effect_select:raw" midi value set above
                        effect_select_from_midi(&mut controller, v)
                            .map(|e| {
                            effect_select_send_controls(&mut controller, &e);
                            e
                        })
                    },
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

                if v != 0 && origin == GUI {
                    let effect_select = controller.get("effect_select:raw").unwrap();
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
                            controller.set("effect_select", 0u16, GUI);
                        }
                    }
                }
            })
        )
    }

    // effect_tweak
    {
        let controller = controller.clone();
        let name = "effect_tweak".to_string();
        callbacks.insert(
            name.clone(),
            Box::new(move || {
                let mut controller = controller.lock().unwrap();
                let (v, origin) = controller.get_origin(&name).unwrap();

                if origin == MIDI {
                    let effect_select = controller.get("effect_select:raw").unwrap();
                    let (entry, _, _) =
                        effect_entry_for_value(&controller.config, effect_select).unwrap();
                    let control_name = &entry.effect_tweak;
                    if control_name.is_empty() {
                        return;
                    }

                    // HACK: as if everything's coming straight from MIDI
                    let mut raw = raw.lock().unwrap();

                    let config = controller.get_config(&name).unwrap();
                    let addr = config.get_addr().unwrap().0 as usize;
                    let val = raw.get(addr).unwrap();

                    let config = controller.get_config(&control_name).unwrap();
                    let addr = config.get_addr().unwrap().0 as usize;
                    raw.set(addr, val, MIDI);
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
    //debug!(target: "animate", "Animate: {:?}?", control_name);
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
            //debug!(target: "animate", "Animate: {:?} for {:?}", c, widget);

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


fn wire_all(controller: Arc<Mutex<Controller>>, raw: Arc<Mutex<Raw>>, objs: &ObjectList) -> Result<Callbacks> {
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
                let adj = scale.adjustment();
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
                        controller.set(&name, adj.value() as u16, GUI);
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
                // HACK: DO NOT PROCESS RADIO BUTTONS HERE!
                if obj.dynamic_cast_ref::<gtk::RadioButton>().is_some() { return }
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
                        controller.set(&name, check.is_active() as u16, GUI);
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
                let group = radio.group();

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
                        if !radio.is_active() { return; }
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
                        combo.active().map(|v| {
                            controller.set(&name, v as u16, GUI);
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
                            let v = controller.get(&name).unwrap() as u16;
                            // TODO: signal_handler_block is a hack because actual value set
                            //       to the UI control is not the same as what came from MIDI,
                            //       so as to not override the MIDI-set value, block the "changed"
                            //       signal handling altogether
                            glib::signal::signal_handler_block(&combo, &signal_id);
                            combo.set_active(Some(v as u32));
                            glib::signal::signal_handler_unblock(&combo, &signal_id);
                        })
                    )
                }
            });
        });

    wire_vol_pedal_position(controller.clone(), objs, callbacks.borrow_mut())?;
    wire_amp_select(controller.clone(), objs, callbacks.borrow_mut())?;
    wire_effect_select(controller, raw, callbacks.borrow_mut())?;

    Ok(callbacks)
}


fn init_all(config: &Config, controller: Arc<Mutex<Controller>>, objs: &ObjectList) -> () {
    for name in &config.init_controls {
        animate(objs, &name, controller.get(&name).unwrap());
    }
}

fn cc_to_control(config: &Config, cc: u8) -> Option<(&String, &Control)> {
    config.controls.iter()
        .find(|&(_, control)| {
            match control.get_cc() {
                Some(v) if v == cc => true,
                _ => false
            }
        })
}

fn cc_to_addr(config: &Config, cc: u8) -> Option<usize> {
    cc_to_control(config, cc)
        .and_then(|(_, control)| control.get_addr())
        .map(|(addr, _)| addr as usize)
}

fn addr_to_control_iter(config: &Config, addr: usize) -> impl Iterator<Item = (&String, &Control)>  {
    config.controls.iter()
        .filter(move |(_, control)| {
            match control.get_addr() {
                // here we specifically disregard the length and concentrate on the
                // first byte of multi-byte controls
                Some((a, _)) if a as usize == addr => true,
                _ => false
            }
        })
}

fn addr_to_cc_iter(config: &Config, addr: usize) -> impl Iterator<Item = u8> + '_ {
    addr_to_control_iter(config, addr)
        .flat_map(|(_, control)| control.get_cc())
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

    let raw = Arc::new(Mutex::new(Raw::new(config.program_size)));

    let controller = Arc::new(Mutex::new(Controller::new(config.clone())));

    let builder = gtk::Builder::from_file("src/pod.glade");
    let objects = ObjectList::new(&builder);
    //objects.dump_debug();

    init_combo(controller.lock().unwrap().deref(), &objects,
               "cab_select", &config.cab_models, |s| s.as_str() )?;
    init_combo(controller.lock().unwrap().deref(), &objects,
               "amp_select", &config.amp_models, |amp| amp.name.as_str() )?;
    init_combo(controller.lock().unwrap().deref(), &objects,
               "effect_select", &config.effects, |eff| eff.name.as_str() )?;

    let callbacks = wire_all(controller.clone(), raw.clone(), &objects)?;

    let window: gtk::Window = builder.object("app_win").unwrap();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // midi ----------------------------------------------------

    // raw / midi reply -> midi out
    {
        let raw = raw.clone();
        let config = config.clone();
        let mut rx = raw.lock().unwrap().subscribe();
        tokio::spawn(async move {
            let make_cc = |idx: usize| -> Option<MidiMessage> {
                addr_to_cc_iter(&config, idx)
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
                  Some(msg) = midi_rx.recv() => {
                        message = Some(msg);
                    },
                  Ok(Event { key: idx, origin: o, .. }) = rx.recv() => {
                        message = make_cc(idx);
                        origin = o;
                    },
                }
                if origin == MIDI || message.is_none() {
                    continue;
                }
                match midi_out.send(&message.unwrap().to_bytes()) {
                    Ok(_) => {}
                    Err(err) => { error!("MIDI OUT error: {}", err); }
                }
            }
        });
    }

    // midi in -> raw / midi loop
    {
        let raw = raw.clone();
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
                        let mut raw = raw.lock().unwrap();

                        let addr = cc_to_addr(&config, control);
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
                let mut control_configs = addr_to_control_iter(&config, addr).peekable();
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

            Continue(true)
        });

    }

    // show the window and do init stuff...
    window.show_all();
    init_all(&config, controller.clone(), &objects);

    debug!("starting gtk main loop");
    gtk::main();

    Ok(())
}
