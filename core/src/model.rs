use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub name: String,
    pub family: u16,
    pub member: u16,
    pub amp_models: Vec<String>,
    pub cab_models: Vec<String>,
    pub controls: HashMap<String, Control>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Control {
    #[serde(rename = "switch")]
    SwitchControl {
        cc: u8
    },
    #[serde(rename = "range")]
    RangeControl {
        cc: u8,
        from: u8,
        to: u8
    }
}