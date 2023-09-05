mod config;
mod module;
pub mod wiring;
pub mod handler;

pub use module::*;

// export some useful functions
pub use config::{gate_threshold_from_midi, gate_threshold_to_midi};
