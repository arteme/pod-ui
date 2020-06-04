use crate::model::Config;
use std::collections::HashMap;

pub struct Controller<'a> {
    config: &'a Config,
    values: HashMap<String, u16>,
}

impl<'a> Controller<'a> {
    pub fn new(config: &'a Config) -> Self {
        let mut values: HashMap<String, u16> = HashMap::new();
        for (name, control) in config.controls.iter() {
            values.insert(name.clone(), 0);
        }

        Controller { config, values }
    }

    pub fn get(&self, name: &str) -> Option<u16> {
        self.values.get(name).cloned()
    }
}