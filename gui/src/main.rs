extern crate gtk;

use anyhow::Context;
use gtk::prelude::*;
use std::fs;

fn main() -> Result<(), anyhow::Error> {
    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let builder = gtk::Builder::new_from_file("src/pod.glade");
    let window: gtk::Window = builder.get_object("w1").unwrap();
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
