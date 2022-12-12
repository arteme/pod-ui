use std::sync::{Arc, Mutex};
use pod_core::edit::EditBuffer;
use pod_core::model::Config;
use pod_core::store::{Signal, StoreSetIm};
use pod_core::store::Origin::MIDI;
use pod_gtk::prelude::*;
use gtk::{Builder, Widget};
use pod_core::handler::BoxedHandler;

use crate::wiring::*;
use crate::config::*;
pub use crate::handler::Pod2Handler;

pub struct Pod2Module;

impl Module for Pod2Module {
    fn config(&self) -> Box<[Config]> {
        vec![POD2_CONFIG.clone(), PODPRO_CONFIG.clone()].into_boxed_slice()
    }

    fn init(&self, config: &'static Config) -> Box<dyn Interface> {
        Box::new(Pod2Interface::new(config))
    }

    fn handler(&self, _config: &'static Config) -> BoxedHandler {
        Box::new(Pod2Handler)
    }
}


struct Pod2Interface {
    config: &'static Config,
    widget: Widget,
    objects: ObjectList
}

impl Pod2Interface {
    fn new(config: &'static Config) -> Self {
        let builder = Builder::from_string(include_str!("pod.glade"));
        let objects = ObjectList::new(&builder);

        let widow: gtk::Window = builder.object("app_win").unwrap();
        let widget = widow.child().unwrap();
        widow.remove(&widget);

        Self { config, widget, objects }
    }
}

impl Interface for Pod2Interface {

    fn widget(&self) -> Widget {
        self.widget.clone()
    }

    fn objects(&self) -> ObjectList {
        self.objects.clone()
    }

    fn wire(&self, edit: Arc<Mutex<EditBuffer>>, callbacks: &mut Callbacks) -> anyhow::Result<()> {
        let config = self.config;
        let controller = edit.lock().unwrap().controller();
        {
            let controller = controller.lock().unwrap();

            init_combo(&controller, &self.objects,
                       "cab_select", &config.cab_models, |s| s.as_str() )?;
            init_combo(&controller, &self.objects,
                       "amp_select", &config.amp_models, |amp| amp.name.as_str() )?;
            init_combo(&controller, &self.objects,
                       "effect_select", &config.effects, |eff| eff.name.as_str() )?;
        }

        wire(controller.clone(), &self.objects, callbacks)?;

        wire_toggles("toggles", &config.toggles,
                     controller.clone(), &self.objects, callbacks)?;
        wire_amp_select(controller.clone(), config, &self.objects, callbacks)?;
        wire_14bit(controller.clone(), &self.objects, callbacks,
                   "delay_time", "delay_time:msb", "delay_time:lsb",
                   false)?;
        wire_effect_select(config, controller, callbacks)?;
        wire_name_change(edit, config, &self.objects, callbacks)?;

        Ok(())
    }

    fn init(&self, edit: Arc<Mutex<EditBuffer>>) -> anyhow::Result<()> {
        let controller = edit.lock().unwrap().controller();
        controller.set_full("reverb_type", 0, MIDI, Signal::Force);

        let digiout_enable = self.config.member == PODPRO_CONFIG.member;
        controller.set_full("digiout_show", digiout_enable as u16, MIDI, Signal::Force);

        Ok(())
    }
}

pub fn module() -> impl Module {
    Pod2Module
}