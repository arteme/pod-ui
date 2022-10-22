use std::sync::{Arc, Mutex};
use pod_core::config::MIDI;
use pod_core::edit::EditBuffer;
use pod_core::model::Config;
use pod_core::store::{Signal, StoreSetIm};
use pod_gtk::prelude::*;
use gtk::{Builder, Widget};
use pod_mod_pod2::wiring::*;

use crate::config;

pub struct PocketPodModule;

impl Module for PocketPodModule {
    fn config(&self) -> Box<[Config]> {
        vec![config::CONFIG.clone()].into_boxed_slice()
    }

    fn init(&self, config: &'static Config) -> Box<dyn Interface> {
        Box::new(PocketPodInterface::new(config))
    }
}

struct PocketPodInterface {
    config: &'static Config,
    widget: Widget,
    objects: ObjectList
}

impl PocketPodInterface {
    fn new(config: &'static Config) -> Self {
        let builder = Builder::from_string(include_str!("pocket-pod.glade"));
        let objects = ObjectList::new(&builder);

        let widow: gtk::Window = builder.object("app_win").unwrap();
        let widget = widow.child().unwrap();
        widow.remove(&widget);

        Self { config, widget, objects }
    }
}

impl Interface for PocketPodInterface {
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

        wire_amp_select(controller.clone(), config, &self.objects, callbacks)?;
        wire_effect_select(config, controller, callbacks)?;
        wire_name_change(edit, config, &self.objects, callbacks)?;
        //todo!()
        Ok(())
    }

    fn init(&self, edit: Arc<Mutex<EditBuffer>>) -> anyhow::Result<()> {
        let controller = edit.lock().unwrap().controller();
        controller.set_full("reverb_type", 0, MIDI, Signal::Force);

        Ok(())
    }
}

pub fn module() -> impl Module {
    PocketPodModule
}