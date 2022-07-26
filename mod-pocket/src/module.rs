use std::sync::{Arc, Mutex};
use pod_core::config::MIDI;
use pod_core::edit::EditBuffer;
use pod_core::model::Config;
use pod_core::store::{Signal, StoreSetIm};
use pod_gtk::*;
use pod_gtk::gtk::prelude::*;
use pod_gtk::gtk::{Builder, Widget};

use crate::config;

pub struct PocketPodModule;

impl Module for PocketPodModule {
    fn config(&self) -> Box<[Config]> {
        vec![config::CONFIG.clone()].into_boxed_slice()
    }

    fn init(&self, config: &'static Config) -> Box<dyn Interface> {
        todo!()
    }
}

struct PocketPodInterface {
    config: &'static Config,
    widget: Widget,
    objects: ObjectList
}

impl PocketPodInterface {
    fn new(config: &'static Config) -> Self {
        let builder = Builder::from_string(include_str!("pod.glade"));
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

    fn wire(&self, edit_buffer: Arc<Mutex<EditBuffer>>, callbacks: &mut Callbacks) -> anyhow::Result<()> {
        //todo!()
        Ok(())
    }

    fn init(&self, edit_buffer: Arc<Mutex<EditBuffer>>) -> anyhow::Result<()> {
        //todo!()
        Ok(())
    }
}

/*
impl Module for PocketPodModule {
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
        wire(config, &self.objects, edit, callbacks)
    }

    fn init(&self, edit: Arc<Mutex<EditBuffer>>) -> anyhow::Result<()> {
        let controller = edit.lock().unwrap().controller();
        controller.set_full("reverb_type", 0, MIDI, Signal::Force);

        Ok(())
    }
}
*/

pub fn module() -> impl Module {
    PocketPodModule
}