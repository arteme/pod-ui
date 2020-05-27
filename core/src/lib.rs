extern crate serde;
extern crate midir;

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate arrayref;

#[macro_use]
extern crate lazy_static;

mod model;
mod midi;
mod util;

pub mod pod;
pub mod config;
