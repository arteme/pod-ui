extern crate gtk;

use anyhow::*;
use gtk::prelude::*;
use glib::Object;
use pod_core::pod::PodConfigs;
use pod_core::controller::Controller;
use log::*;
use std::sync::{Arc, Mutex};
use pod_core::model::{Config, Control};

fn clamp(v: f64) -> u16 {
    if v.is_nan() { 0 } else {
        if v.is_sign_negative() { 0 } else {
            if v > 0xffff as f64 { 0xffff } else { v as u16 }
        }
    }
}

fn wire_all(controller: Arc<Mutex<Controller>>, objs: &[Object]) -> Result<()> {
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


                    adj.connect_value_changed(move |adj| {
                        let mut controller = controller.lock().unwrap();
                        controller.set(&name, adj.get_value() as u16);
                    })
                });

            });

    for obj in objs {
        let name = object_name(obj);
        if name.is_none() { continue; }


        println!("{:?}", object_name(obj));
    }


    Ok(())
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
    wire_all(controller, &objects)?;
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
    gtk::main();
    /*
    loop {
        gtk::main_iteration_do(false);
        sleep_ms(1);
    }
     */


    Ok(())
}
