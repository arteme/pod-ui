// Visual dev tool: renders a HOCON device layout file in a plain GTK window,
// with no MIDI/controller wiring, so a `.conf` file can be eyeballed without
// needing a real device.
use std::{env, fs, process, time::Duration};

use pod_gtk::prelude::*;

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: preview <path-to-conf-file> [screenshot-output-path]");
        process::exit(1);
    };
    let screenshot_path = args.next();

    let conf = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    gtk::init().expect("gtk::init failed");

    let widget = pod_gtk::build_ui(&conf).expect("build_ui failed");

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(&format!("preview: {}", path));
    window.add(&widget);
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Propagation::Stop
    });
    window.show_all();

    if let Some(screenshot_path) = screenshot_path {
        let window = window.clone();
        // give the window manager/X server a moment to actually map and paint
        // the window before grabbing pixels off it
        glib::timeout_add_local_once(Duration::from_millis(500), move || {
            let gdk_window = window.window().expect("window not realized");
            let (width, height) = (gdk_window.width(), gdk_window.height());
            let pixbuf = gdk_window.pixbuf(0, 0, width, height)
                .expect("failed to capture window pixbuf");
            pixbuf.savev(&screenshot_path, "png", &[])
                .unwrap_or_else(|e| panic!("failed to save screenshot to {:?}: {}", screenshot_path, e));
            gtk::main_quit();
        });
    }

    gtk::main();
}
