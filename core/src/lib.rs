extern crate serde;
extern crate midir;

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate arrayref;

pub mod midi;
mod util;

pub mod store;
pub mod model;
pub mod pod;
pub mod config;
pub mod controller;
pub mod program;
pub mod raw;
pub mod strings;
pub mod names;
pub mod dump;
