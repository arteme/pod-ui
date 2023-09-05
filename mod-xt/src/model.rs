use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct StompConfig {
    pub name: String,
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct ModConfig {
    pub name: String,
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct DelayConfig {
    pub name: String,
    pub labels: HashMap<String, String>,
}

// common config access trait

pub trait ConfigAccess {
    fn name(&self) -> &String;
    fn labels(&self) -> &HashMap<String, String>;
}

impl ConfigAccess for ModConfig {
    fn name(&self) -> &String {
        &self.name
    }

    fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

impl ConfigAccess for StompConfig {
    fn name(&self) -> &String {
        &self.name
    }

    fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

impl ConfigAccess for DelayConfig {
    fn name(&self) -> &String {
        &self.name
    }

    fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}
