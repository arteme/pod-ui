pub use glib;
pub use gtk;
pub use gtk::prelude::*;

pub use crate::*;

/// A prelude that includes everything needed to write GTK widgets
pub mod subclass {
    pub use super::*;
    pub use gtk::subclass::prelude::*;

    pub use glib::{ParamSpec, Value};
    pub use glib::value::FromValue;
}