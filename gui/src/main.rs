extern crate gtk;

mod opts;

use anyhow::*;
use gtk::prelude::*;
use glib::Object;
use pod_core::pod::{MidiIn, MidiOut, PodConfigs};
use pod_core::controller::Controller;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{Config, Control, GetCC};
use std::collections::HashMap;
use crate::opts::*;
use pod_core::midi::{MidiResponse, MidiMessage};
use tokio::sync::broadcast::RecvError;
use std::borrow::BorrowMut;
use std::ops::Deref;

fn clamp(v: f64) -> u16 {
    if v.is_nan() { 0 } else {
        if v.is_sign_negative() { 0 } else {
            if v > 0xffff as f64 { 0xffff } else { v as u16 }
        }
    }
}

type Callbacks = HashMap<String, Box<dyn Fn() -> ()>>;

fn obj_by_name(objs: &[Object], name: &str) -> Result<Object> {
    objs.iter()
        .find(|o|
            o.get_property("name")
                .map(|p| p.get::<String>().unwrap())
                .unwrap_or(None)
                .unwrap_or("".into()) == name)
        .with_context(|| format!("Object not found by name {:?}", name))
        .map(|obj| obj.clone())
}

fn ref_by_name<T: ObjectType>(objs: &[Object], name: &str) -> Result<T> {
    let obj = obj_by_name(objs, name)?;
    let cast = obj.dynamic_cast_ref::<T>()
        .with_context(|| format!("Object by name {:?} is can not be cast to type {:?}", name, T::static_type()))?
        .clone();
    Ok(cast)
}


fn wire_vol_pedal_position(controller: Arc<Mutex<Controller>>, objs: &[Object], callbacks: &mut Callbacks) -> Result<()> {
    let name = "vol_pedal_position".to_string();
    let vol_pedal_position = ref_by_name::<gtk::Button>(objs, &name)?;
    let amp_enable = ref_by_name::<gtk::Widget>(objs, "amp_enable")?;
    let volume_enable = ref_by_name::<gtk::Widget>(objs, "volume_enable")?;

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

fn wire_amp_select(controller: Arc<Mutex<Controller>>, objs: &[Object], callbacks: &mut Callbacks) -> Result<()> {
    let presence_widget = ref_by_name::<gtk::Widget>(objs, "presence")?;
    let presence_label_widget = ref_by_name::<gtk::Label>(objs, "presence_label")?;

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

fn init_cab_select(config: &Config, controller: &Controller, objs: &[Object]) -> Result<()> {
    let select = ref_by_name::<gtk::ComboBoxText>(objs, "cab_select")?;
    for name in config.cab_models.iter() {
        select.append_text(name.as_str());
    }

    let v = controller.get("cab_select").unwrap();
    select.set_active(Some(v as u32));

    Ok(())
}

fn init_amp_select(config: &Config, controller: &Controller, objs: &[Object]) -> Result<()> {
    let select = ref_by_name::<gtk::ComboBoxText>(objs, "amp_select")?;
    for amp in config.amp_models.iter() {
        select.append_text(amp.name.as_str());
    }

    let v = controller.get("amp_select").unwrap();
    select.set_active(Some(v as u32));

    Ok(())
}


fn wire_all(controller: Arc<Mutex<Controller>>, objs: &[Object]) -> Result<Callbacks> {
    let mut callbacks = Callbacks::new();

    let object_name = | o: &Object | o.get_property("name")
        .map(|p| p.get::<String>().unwrap())
        .unwrap_or(None)
        .filter(|v| !v.is_empty());

    objs.iter()
        .flat_map(|obj| object_name(obj).map(|name| (obj, name)) )
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
                        let mut controller = controller.lock().unwrap();
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
                            let v = {
                                let controller = controller.lock().unwrap();
                                controller.get(&name).unwrap()
                            };
                            check.set_active(v > 0);
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
                            let mut controller = controller.lock().unwrap();
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
                            let v = {
                                let controller = controller.lock().unwrap();
                                controller.get(&name).unwrap()
                            };
                            combo.set_active(Some(v as u32));
                        })
                    )
                }
            });
        });

    wire_vol_pedal_position(controller.clone(), objs, callbacks.borrow_mut())?;
    wire_amp_select(controller, objs, callbacks.borrow_mut())?;

    for obj in objs {
        let name = object_name(obj);
        if name.is_none() { continue; }

        println!("{:?}", object_name(obj));
    }

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

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let configs = PodConfigs::new()?;
    let config: Config = configs.by_name(&"POD 2.0".into()).context("Config not found by name 'POD 2.0'")?;
    let controller = Arc::new(Mutex::new(Controller::new(config.clone())));

    let builder = gtk::Builder::new_from_file("src/pod.glade");
    let objects = builder.get_objects();

    init_cab_select(&config, controller.lock().unwrap().deref(), &objects)?;
    init_amp_select(&config, controller.lock().unwrap().deref(), &objects)?;

    let callbacks = wire_all(controller.clone(), &objects)?;

    let window: gtk::Window = builder.get_object("app_win").unwrap();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // midi ----------------------------------------------------
    {
        let controller = controller.clone();
        let mut rx = {
            let controller = controller.lock().unwrap();
            controller.subscribe()
        };
        tokio::spawn(async move {
            loop {
                let name = match rx.recv().await {
                    Ok(name) => name,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return Ok(())
                };
                let (config, val) = {
                    let controller = controller.lock().unwrap();
                    let config = controller.get_config(&name).unwrap();
                    let val = controller.get(&name).unwrap();
                    (config.clone(), val)
                };
                let scale= match &config {
                    Control::SwitchControl(_) => 64u16,
                    Control::RangeControl(c) => 127 / c.to as u16,
                    _ => 1
                };
                let message = MidiMessage::ControlChange { channel: 1, control: config.get_cc().unwrap(), value: (val * scale) as u8 };
                midi_out.send(&message.to_bytes()).unwrap();
            }
            Err(anyhow!("Never reached")) // helps with inferring E for Result<T,E>
        });
    }

    {
        let controller = controller.clone();
        tokio::spawn(async move {
            loop {
                let data = midi_in.recv().await;
                if data.is_none() {
                    return Ok(()); // shutdown
                }
                let event = MidiResponse::from_bytes(data.unwrap())?;
                match event {
                    MidiResponse::ControlChange { channel: _, control, value } => {
                        let mut controller = controller.lock().unwrap();
                        let (name, config) = controller.get_config_by_cc(control).unwrap();
                        let name = name.clone();
                        let scale= match &config {
                            Control::SwitchControl(_) => 64u16,
                            Control::RangeControl(c) => 127 / c.to as u16,
                            _ => 1
                        };
                        controller.set(&name, value as u16 / scale);
                    }

                    _ => {} //Err(anyhow!("Incorrect MIDI response"))
                }
            }
            Err(anyhow!("Never reached")) // helps with inferring E for Result<T,E>
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
                    None => { panic!("WTF"); },
                    Some(cb) => cb(),
                }
            },
            Err(_) => {},
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
