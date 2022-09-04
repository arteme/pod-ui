use crate::model::Amp;
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

pub mod shorthand {
    use super::*;

    pub fn amp(name: &str) -> AmpBuilder {
        AmpBuilder::new(name)
    }
}