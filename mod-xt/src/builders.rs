use std::collections::HashMap;
use crate::model::*;

struct GenericConfigBuilder {
    name: String,
    prefix: String,
    labels: HashMap<String, String>,
    n: usize
}

impl GenericConfigBuilder {
    pub fn new(name: &str, prefix: &str) -> Self {
        Self {
            name: name.into(),
            prefix: prefix.into(),
            labels: HashMap::new(),
            n: 2
        }
    }
}

pub trait GenericConfigBuilderOps {
    fn add(&mut self, control: &str, label: &str);

    fn control(&mut self, name: &str) -> &mut Self {
        self.add("", name);
        self
    }

    fn skip(&mut self) -> &mut Self {
        self.add("", "");
        self
    }
}

impl GenericConfigBuilderOps for GenericConfigBuilder {
    fn add(&mut self, control: &str, label: &str) {
        if !label.is_empty() {
            let control = if !control.is_empty() {
                format!("{}_param{}_{}", &self.prefix, self.n, control)
            } else {
                format!("{}_param{}", &self.prefix, self.n)
            };

            self.labels.insert(control, label.into());
        }
        self.n += 1;
    }
}

// ---------------------------

pub struct StompConfigBuilder(GenericConfigBuilder);

impl GenericConfigBuilderOps for StompConfigBuilder {
    fn add(&mut self, control: &str, label: &str) {
        self.0.add(control, label)
    }
}

impl StompConfigBuilder {
    pub fn new(name: &str) -> Self {
        Self(GenericConfigBuilder::new(name, "stomp"))
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

    pub fn build(&self) -> StompConfig {
        StompConfig { name: self.0.name.clone(), labels: self.0.labels.clone() }
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

pub struct ModConfigBuilder(GenericConfigBuilder);

impl GenericConfigBuilderOps for ModConfigBuilder {
    fn add(&mut self, control: &str, label: &str) {
        self.0.add(control, label)
    }
}

impl ModConfigBuilder {
    pub fn new(name: &str) -> Self {
        Self(GenericConfigBuilder::new(name, "mod"))
    }

    pub fn wave(&mut self, name: &str) -> &mut Self {
        self.add("wave", name);
        self
    }

    pub fn build(&self) -> ModConfig {
        ModConfig { name: self.0.name.clone(), labels: self.0.labels.clone() }
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

pub struct DelayConfigBuilder(GenericConfigBuilder);

impl GenericConfigBuilderOps for DelayConfigBuilder {
    fn add(&mut self, control: &str, label: &str) {
        self.0.add(control, label)
    }
}

impl DelayConfigBuilder {
    pub fn new(name: &str) -> Self {
        Self(GenericConfigBuilder::new(name, "delay"))
    }

    pub fn heads(&mut self, name: &str) -> &mut Self {
        self.add("heads", name);
        self
    }

    pub fn bits(&mut self, name: &str) -> &mut Self {
        self.add("bits", name);
        self
    }

    pub fn build(&self) -> DelayConfig {
        DelayConfig { name: self.0.name.clone(), labels: self.0.labels.clone() }
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

