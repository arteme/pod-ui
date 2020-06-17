extern crate gtk;

use anyhow::*;
use gtk::prelude::*;
use glib::Object;
use pod_core::pod::PodConfigs;
use pod_core::controller::Controller;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{Config, Control};
use std::collections::HashMap;

fn clamp(v: f64) -> u16 {
    if v.is_nan() { 0 } else {
        if v.is_sign_negative() { 0 } else {
            if v > 0xffff as f64 { 0xffff } else { v as u16 }
        }
    }
}

type Callbacks = HashMap<String, Box<dyn Fn() -> ()>>;

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
                    // wire gtk::Scale and its internal gtk::Adjustment
                    let adj = scale.get_adjustment();
                    info!("adj {:?}", adj);
                    let controller = controller.clone();
                    let rx;
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

                        rx = controller.subscribe();
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

            });

    for obj in objs {
        let name = object_name(obj);
        if name.is_none() { continue; }


        println!("{:?}", object_name(obj));
    }

    Ok(callbacks)
}


fn main() -> Result<()> {
    simple_logger::init()?;
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
