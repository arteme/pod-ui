extern crate gtk;

use anyhow::*;
use gtk::prelude::*;
use glib::Object;
use pod_core::pod::PodConfigs;
use pod_core::controller::Controller;
//use glib::Object;

fn wire_all(objs: &[Object]) -> Result<()> {
    let object_name = | o: &Object | o.get_property("name")
        .map(|p| p.get::<String>().unwrap())
        .unwrap_or(None)
        .filter(|v| !v.is_empty());

    for obj in objs {
        println!("{:?}", object_name(obj));
    }


    Ok(())
}


fn main() -> Result<()> {
    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let configs = PodConfigs::new()?;
    let config = configs.by_name(&"POD 2.0".into()).context("Config not found by name 'POD 2.0'")?;
    let controller = Controller::new(config);

    let builder = gtk::Builder::new_from_file("src/pod.glade");
    let objects = builder.get_objects();
    wire_all(&objects)?;
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
