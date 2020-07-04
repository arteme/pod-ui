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

fn clamp(v: f64) -> u16 {
    if v.is_nan() { 0 } else {
        if v.is_sign_negative() { 0 } else {
            if v > 0xffff as f64 { 0xffff } else { v as u16 }
        }
    }
}

type Callbacks = HashMap<String, Box<dyn Fn() -> ()>>;

fn obj_by_name(objs: &[Object], name: &str) -> Object {
    objs.iter()
        .find(|o|
            o.get_property("name")
                .map(|p| p.get::<String>().unwrap())
                .unwrap_or(None)
                .unwrap_or("".into()) == name)
        .unwrap()
        .clone()
}

fn ref_by_name<T: ObjectType>(objs: &[Object], name: &str) -> T {
    obj_by_name(objs, name).dynamic_cast_ref::<T>().unwrap().clone()
}


fn wire_volume_pedal_location(controller: Arc<Mutex<Controller>>, objs: &[Object], callbacks: &mut Callbacks) {
    let volume_pedal_location = ref_by_name::<gtk::Button>(objs, "volume_pedal_location_button");
    let amp_enable = ref_by_name::<gtk::Widget>(objs, "amp_enable");
    let volume_enable = ref_by_name::<gtk::Widget>(objs, "volume_enable");

    let set_in_order = {
        let volume_pedal_location = volume_pedal_location.clone();

        move |volume_pre_amp: bool| {
            let ancestor = amp_enable.get_ancestor(gtk::Grid::static_type()).unwrap();
            let grid = ancestor.dynamic_cast_ref::<gtk::Grid>().unwrap();
            grid.remove(&amp_enable);
            grid.remove(&volume_enable);

            let (volume_left, amp_left) = match volume_pre_amp {
                true => {
                    volume_pedal_location.set_label(">");
                    (1, 2)
                },
                false => {
                    volume_pedal_location.set_label("<");
                    (2, 1)
                }
            };
            grid.attach(&amp_enable, amp_left, 1, 1, 1);
            grid.attach(&volume_enable, volume_left, 1, 1, 1);
        }
    };

    set_in_order(true);

    // gui -> controller
    {
        let controller = controller.clone();
        volume_pedal_location.connect_clicked(move |_| {
            let mut controller = controller.lock().unwrap();
            let v = controller.get("volume_pedal_location").unwrap() < 64; // 0..63 -- volume pre tube drive
            let v = !v; // toggling
            controller.set("volume_pedal_location", if v { 0 } else { 64 });
        });
    }

    // controller -> gui
    {
        let controller = controller.clone();
        let name = "volume_pedal_location".to_string();
        callbacks.insert(
            name.clone(),
            Box::new(move || {
                let v = {
                    let controller = controller.lock().unwrap();
                    controller.get(&name).unwrap()
                };
                set_in_order(v < 64);
            })
        )
    };

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
                        controller.set(&name, if check.get_active() { 64 } else { 0 });
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
                            check.set_active(v > 63);
                        })
                    )
                }
            });
        });

    wire_volume_pedal_location(controller, objs, callbacks.borrow_mut());

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
    let controller = Arc::new(Mutex::new(Controller::new(config)));

    let builder = gtk::Builder::new_from_file("src/pod.glade");
    let objects = builder.get_objects();
    let callbacks = wire_all(controller.clone(), &objects)?;
    /*
    for o in objects {
        println!("{:?}", o);
        for p in o.list_properties() {
            println!(" - {:?} {:?}", p.get_name(), p.get_value_type().name());
        }
        //let id: Option<String> = if o.has_property("id", None).is_ok() { o.get_property("id")?.get()? } else { None };
        //println!("{:?} {:?}", id, o);
    }
     */

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
                let message = MidiMessage::ControlChange { channel: 1, control: config.get_cc().unwrap(), value: val as u8 };
                midi_out.send(&message.to_bytes());
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
                        let (name, _) = controller.get_config_by_cc(control).unwrap();
                        let name = name.clone();
                        controller.set(&name, value as u16);
                    }

                    /*
                MidiResponse::C { channel: _, family, member, ver: _ } => {
                    let pod = PODS().iter().find(|config| {
                        family == config.family && member == config.member
                    }).unwrap();
                    info!("Discovered: {}", pod.name);
                    Ok(pod)
                }

                 */
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

    gtk::main();

    /*
    loop {
        gtk::main_iteration_do(false);
        sleep_ms(1);
    }
     */

    Ok(())
}
