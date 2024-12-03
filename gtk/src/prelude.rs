pub use glib;
pub use gtk;
pub use gdk;
pub use glib::prelude::*;
pub use gtk::prelude::*;
pub use gdk::prelude::*;

pub use gtk::gio;
//pub use gtk::gio::prelude::*;

pub use glib::ControlFlow;
pub use glib::Propagation;

pub use crate::*;

/// A prelude that includes everything needed to write GTK widgets
pub mod subclass {
    pub use super::*;
    pub use gtk::subclass::prelude::*;
    pub use glib::subclass::prelude::*;

    pub use glib::{ParamSpec, Value};
    pub use glib::value::FromValue;
}