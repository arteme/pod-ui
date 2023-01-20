use std::collections::HashMap;
use crate::model::*;

pub struct StompConfigBuilder {
    name: String,
    labels: HashMap<String, String>,
    n: usize
}

impl StompConfigBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            labels: HashMap::new(),
            n: 2
        }
    }

    fn add(&mut self, control: &str, label: &str) {
        if !label.is_empty() {
            let control = if !control.is_empty() {
                format!("stomp_param{}_{}", self.n, control)
            } else {
                format!("stomp_param{}", self.n)
            };

            self.labels.insert(control, label.into());
        }
        self.n += 1;
    }

    pub fn wave(&mut self, name: &str) -> &mut Self {
        self.add("wave", name);
        self
    }

    pub fn octave(&mut self, name: &str) -> &mut Self {
        self.add("octave", name);
        self
    }

    pub fn offset(&mut self, name: &str) -> &mut Self {
        self.add("offset", name);
        self
    }

    pub fn control(&mut self, name: &str) -> &mut Self {
        self.add("", name);
        self
    }

    pub fn skip(&mut self) -> &mut Self {
        self.add("", "");
        self
    }

    pub fn build(&self) -> StompConfig {
        StompConfig { name: self.name.clone(), labels: self.labels.clone() }
    }
}

impl Into<StompConfig> for &mut StompConfigBuilder {
    fn into(self) -> StompConfig {
        self.build()
    }
}

pub fn stomp(name: &str) -> StompConfigBuilder {
    StompConfigBuilder::new(name)
}

// -----------------------------------------------------------------

pub struct ModConfigBuilder {
    name: String,
    labels: HashMap<String, String>,
    n: usize
}

impl ModConfigBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            labels: HashMap::new(),
            n: 2
        }
    }

    fn add(&mut self, label: &str) {
        if !label.is_empty() {
            let control = format!("mod_param{}", self.n);
            self.labels.insert(control, label.into());
        }
        self.n += 1;
    }

    pub fn control(&mut self, name: &str) -> &mut Self {
        self.add(name);
        self
    }

    pub fn skip(&mut self) -> &mut Self {
        self.add("");
        self
    }

    pub fn build(&self) -> ModConfig {
        ModConfig { name: self.name.clone(), labels: self.labels.clone() }
    }
}

impl Into<ModConfig> for &mut ModConfigBuilder {
    fn into(self) -> ModConfig {
        self.build()
    }
}

pub fn modc(name: &str) -> ModConfigBuilder {
    ModConfigBuilder::new(name)
}

// -----------------------------------------------------------------

pub struct DelayConfigBuilder {
    name: String,
    labels: HashMap<String, String>,
    n: usize
}

impl DelayConfigBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            labels: HashMap::new(),
            n: 2
        }
    }

    fn add(&mut self, control: &str, label: &str) {
        if !label.is_empty() {
            let control = if !control.is_empty() {
                format!("delay_param{}_{}", self.n, control)
            } else {
                format!("delay_param{}", self.n)
            };

            self.labels.insert(control, label.into());
        }
        self.n += 1;
    }

    pub fn heads(&mut self, name: &str) -> &mut Self {
        self.add("heads", name);
        self
    }

    pub fn bits(&mut self, name: &str) -> &mut Self {
        self.add("bits", name);
        self
    }

    pub fn control(&mut self, name: &str) -> &mut Self {
        self.add("", name);
        self
    }

    pub fn skip(&mut self) -> &mut Self {
        self.add("", "");
        self
    }

    pub fn build(&self) -> DelayConfig {
        DelayConfig { name: self.name.clone(), labels: self.labels.clone() }
    }
}

impl Into<DelayConfig> for &mut DelayConfigBuilder {
    fn into(self) -> DelayConfig {
        self.build()
    }
}

pub fn delay(name: &str) -> DelayConfigBuilder {
    DelayConfigBuilder::new(name)
}

