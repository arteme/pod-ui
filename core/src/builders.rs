use crate::model::{Amp, Toggle};
use crate::util::def;

pub struct AmpBuilder(Amp);

impl AmpBuilder {
    pub fn new(name: &str) -> Self {
        let amp = Amp { name: name.into(), ..def() };
        Self(amp)
    }

    pub fn bright(&mut self) -> &mut Self {
        self.0.bright_switch = true;
        self
    }

    pub fn presence(&mut self) -> &mut Self {
        self.0.presence = true;
        self
    }

    pub fn delay2(&mut self) -> &mut Self {
        self.0.drive2 = true;
        self
    }

    pub fn room(&mut self) -> &mut Self {
        self.0.reverb = 1;
        self
    }

    pub fn spring(&mut self) -> &mut Self {
        self.0.reverb = 0;
        self
    }

    pub fn build(&self) -> Amp {
        self.0.clone()
    }
}

impl Into<Amp> for &mut AmpBuilder {
    fn into(self) -> Amp {
        self.build()
    }
}

impl Into<Amp> for AmpBuilder {
    fn into(self) -> Amp {
        self.build()
    }
}

// -------------------------------------------------------------

pub struct ToggleBuilder(Toggle);

impl ToggleBuilder {
    pub fn new(name: &str) -> Self {
        let toggle = Toggle { name: name.into(), ..def() };
        Self(toggle)
    }

    pub fn non_moving(&mut self, position: usize) -> &mut Self {
        self.0.position_control = "".into();
        self.0.on_position = position;
        self.0.off_position = position;
        self
    }

    pub fn moving(&mut self, position_control: &str,
                  on_position: usize, off_position: usize) -> &mut Self {
        self.0.position_control = position_control.into();
        self.0.on_position = on_position;
        self.0.off_position = off_position;
        self
    }

    pub fn build(&self) -> Toggle {
        self.0.clone()
    }
}

impl Into<Toggle> for &mut ToggleBuilder {
    fn into(self) -> Toggle {
        self.build()
    }
}

// -------------------------------------------------------------

pub mod shorthand {
    use super::*;

    pub fn amp(name: &str) -> AmpBuilder {
        AmpBuilder::new(name)
    }

    pub fn toggle(name: &str) -> ToggleBuilder {
        ToggleBuilder::new(name)
    }
}