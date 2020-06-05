extern crate serde;
extern crate midir;

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate arrayref;
#[macro_use]
extern crate maplit;

mod midi;
mod util;

pub mod model;
pub mod pod;
pub mod config;
pub mod controller;
