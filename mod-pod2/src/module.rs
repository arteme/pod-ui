use std::sync::{Arc, Mutex};
use pod_core::config::MIDI;
use pod_core::edit::EditBuffer;
use pod_core::model::Config;
use pod_core::store::{Signal, StoreSetIm};
use pod_gtk::*;
use pod_gtk::gtk::prelude::*;
use pod_gtk::gtk::{Builder, Widget};
use crate::config::CONFIG;

use crate::wiring::*;

struct Pod2Module {
    widget: Widget,
    objects: ObjectList
}

impl Pod2Module {
    fn new() -> Self {
        let builder = Builder::from_string(include_str!("pod.glade"));
        let objects = ObjectList::new(&builder);

        let widow: gtk::Window = builder.object("app_win").unwrap();
        let widget = widow.child().unwrap();
        widow.remove(&widget);

        Pod2Module { widget, objects }
    }
}

impl Module for Pod2Module {
    fn config(&self) -> Config {
        CONFIG.clone()
    }

    fn widget(&self) -> Widget {
        self.widget.clone()
    }

    fn objects(&self) -> ObjectList {
        self.objects.clone()
    }

    fn wire(&self, edit: Arc<Mutex<EditBuffer>>, callbacks: &mut Callbacks) -> anyhow::Result<()> {
        let config = &*CONFIG;
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

        wire_vol_pedal_position(controller.clone(), &self.objects, callbacks)?;
        wire_amp_select(controller.clone(), &config, &self.objects, callbacks)?;
        wire_effect_select(controller, callbacks)?;
        wire_name_change(edit, &config, &self.objects, callbacks)?;

        Ok(())
    }

    fn init(&self, edit: Arc<Mutex<EditBuffer>>) -> anyhow::Result<()> {
        let controller = edit.lock().unwrap().controller();
        controller.set_full("reverb_type", 0, MIDI, Signal::Force);

        Ok(())
    }
}

pub fn module() -> impl Module {
    Pod2Module::new()
}