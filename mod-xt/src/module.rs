use std::sync::{Arc, Mutex};
use pod_core::edit::EditBuffer;
use pod_core::model::Config;
use pod_core::store::{Signal, StoreSetIm};
use pod_gtk::prelude::*;
use gtk::{Builder, Widget};
use pod_core::handler::BoxedHandler;
use pod_core::store::Origin::MIDI;
use pod_mod_pod2::wiring::*;

use crate::config;
use crate::handler::PodXtHandler;
use crate::widgets::Tuner;
use crate::wiring::{*, init_combo};

pub struct PodXtModule;

impl Module for PodXtModule {
    fn config(&self) -> Box<[Config]> {
        vec![
            config::PODXT_CONFIG.clone(),
            config::PODXT_PRO_CONFIG.clone(),
            config::PODXT_LIVE_CONFIG.clone(),
        ].into_boxed_slice()
    }

    fn init(&self, config: &'static Config) -> Box<dyn Interface> {
        Box::new(PodXtInterface::new(config))
    }

    fn handler(&self, config: &'static Config) -> BoxedHandler {
        Box::new(PodXtHandler::new(config))
    }
}

struct PodXtInterface {
    config: &'static Config,
    widget: Widget,
    objects: ObjectList
}

impl PodXtInterface {
    fn new(config: &'static Config) -> Self {
        let builder = Builder::from_string(include_str!("pod-xt.glade"));
        let objects = ObjectList::new(&builder);

        let widow: gtk::Window = builder.object("app_win").unwrap();
        let widget = widow.child().unwrap();
        widow.remove(&widget);

        Self { config, widget, objects }
    }
}

impl Interface for PodXtInterface {
    fn widget(&self) -> Widget {
        self.widget.clone()
    }

    fn objects(&self) -> ObjectList {
        self.objects.clone()
    }

    fn wire(&self, edit: Arc<Mutex<EditBuffer>>, callbacks: &mut Callbacks) -> anyhow::Result<()> {
        let config = self.config;
        let controller = edit.lock().unwrap().controller();

        init_combo(&self.objects, "amp_select",
                   &config.amp_models, |c| c.name.as_str())?;
        init_combo(&self.objects, "cab_select",
                   &config.cab_models, |v| v.as_str())?;
        init_combo(&self.objects, "mic_select",
                   &config::MIC_NAMES, |v| v.as_str())?;
        init_combo(&self.objects, "reverb_select",
                   &config::REVERB_NAMES, |s| s.as_str())?;
        init_combo(&self.objects, "stomp_select",
                   &config::STOMP_CONFIG, |c| c.name.as_str())?;
        init_combo(&self.objects, "mod_select",
                   &config::MOD_CONFIG, |c| c.name.as_str())?;
        init_combo(&self.objects, "mod_note_select",
                   &config::NOTE_NAMES, |v| v.as_str())?;
        init_combo(&self.objects, "delay_select",
                   &config::DELAY_CONFIG, |c| c.name.as_str())?;
        init_combo(&self.objects, "delay_note_select",
                   &config::NOTE_NAMES, |v| v.as_str())?;
        init_combo(&self.objects, "wah_select",
                   &config::WAH_NAMES, |s| s.as_str())?;
        init_combo(&self.objects, "tweak_param_select",
                   &config::TWEAK_PARAM_NAMES, |s| s.as_str())?;
        init_combo(&self.objects, "pedal_assign_select",
                   &config::PEDAL_ASSIGN_NAMES, |s| s.as_str())?;

        wire(controller.clone(), &self.objects, callbacks)?;

        wire_toggles("toggles", &config.toggles,
                     controller.clone(), &self.objects, callbacks)?;
        wire_stomp_select(controller.clone(), &self.objects, callbacks)?;
        wire_mod_select(controller.clone(), &self.objects, callbacks)?;
        wire_delay_select(controller.clone(), &self.objects, callbacks)?;
        wire_14bit(controller.clone(), &self.objects, callbacks,
                   "mod_speed", "mod_speed:msb", "mod_speed:lsb",
                   true)?;
        wire_14bit(controller.clone(), &self.objects, callbacks,
                   "delay_time", "delay_time:msb", "delay_time:lsb",
                   true)?;
        wire_di_show(controller.clone(), config, &self.objects, callbacks)?;
        wire_xt_packs(controller.clone(), &self.objects, callbacks)?;
        wire_mics_update(controller.clone(), config, &self.objects, callbacks)?;
        wire_pedal_assign(controller.clone(), &self.objects, callbacks)?;
        wire_name_change(edit, config, &self.objects, callbacks)?;
        resolve_footswitch_mode_show(&self.objects, config)?;

        let tuner_box = self.objects.ref_by_name::<gtk::Box>("tuner_box").unwrap();
        let tuner = Tuner::new();
        tuner_box.add(&tuner);
        tuner.show();
        wire_tuner(tuner, controller.clone(), &self.objects, callbacks)?;

        Ok(())
    }

    fn init(&self, edit: Arc<Mutex<EditBuffer>>) -> anyhow::Result<()> {
        let controller = edit.lock().unwrap().controller();

        controller.set_full("amp_enable", 1, MIDI, Signal::Force);
        controller.set_full("di:show", 0, MIDI, Signal::Force);
        // say we have all packs, unless a real POD tells us otherwise
        controller.set_full("xt_packs", 0xf, MIDI, Signal::Force);

        let show = self.config.member == config::PODXT_PRO_CONFIG.member;
        controller.set_full("loop_enable:show", show as u16, MIDI, Signal::Force);

        let show = self.config.member == config::PODXT_LIVE_CONFIG.member;
        controller.set_full("footswitch_mode:show", show as u16, MIDI, Signal::Force);

        Ok(())
    }
}

pub fn module() -> impl Module {
    PodXtModule
}