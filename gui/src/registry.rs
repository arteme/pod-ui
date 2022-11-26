use std::sync::{Arc, Mutex};
use anyhow::*;
use log::error;
use pod_core::config::register_config;
use pod_core::dump::ProgramsDump;
use pod_core::edit::EditBuffer;
use pod_core::handler::BoxedHandler;
use pod_core::model::{AbstractControl, Config};
use pod_core::store::{Signal, Store};
use pod_core::store::Origin::MIDI;
use pod_gtk::prelude::*;

static mut MODULES: Vec<Box<dyn Module>> = vec![];

fn validate_unique_cc(config: &Config) -> bool {
    let mut seen_cc = vec![];
    let mut ok = true;
    for (k, v) in config.controls.iter() {
        if let Some(cc) = v.get_cc() {
            if seen_cc.contains(&cc) {
                error!("Config {:?} contains multiple controls for CC={}", config.name, cc);
                ok = false;
            }
            seen_cc.push(cc);
        }
    }

    ok
}

pub fn register_module(module: impl Module + 'static) -> Result<()> {
    for config in module.config().iter() {
        if !validate_unique_cc(config) {
            bail!("Config {:?} failed unique CC validation!", config.name);
        }
        register_config(config);
    }

    unsafe {
        MODULES.push(Box::new(module))
    }

    Ok(())
}

pub fn module_for_config(config: &Config) -> Option<&Box<dyn Module>> {
    unsafe {
        for module in MODULES.iter() {
            for c in module.config().iter() {
                if *c == *config {
                    return Some(module);
                }
            }
        }
    }

    None
}

pub struct InitializedInterface {
    pub handler: BoxedHandler,
    pub edit_buffer: Arc<Mutex<EditBuffer>>,
    pub dump: Arc<Mutex<ProgramsDump>>,
    pub callbacks: Callbacks,
    pub widget: gtk::Widget,
    pub objects: ObjectList
}

pub fn init_module(config: &'static Config) -> Result<InitializedInterface> {
    let module = module_for_config(config).unwrap();
    let interface = module.init(config);
    let handler = module.handler(config);

    let edit_buffer = Arc::new(Mutex::new(EditBuffer::new(config)));
    let dump = Arc::new(Mutex::new(ProgramsDump::new(config)));
    let mut callbacks = Callbacks::new();

    let widget = interface.widget();
    let objects = interface.objects();

    interface.wire(edit_buffer.clone(), &mut callbacks)?;
    interface.init(edit_buffer.clone())?;

    // TODO: `init_controls` below only get an animate() call, while `module.init()`
    //       sets 0 to the controller. We can unify all init as setting 0 to the controller
    //       (gotta ensure this doesn't leak to MIDI layer) thus making `module.init()` obsolete.

    // init module controls
    let edit_buffer_guard = edit_buffer.lock().unwrap();
    let controller = edit_buffer_guard.controller_locked();
    for name in &config.init_controls {
        let value = controller.get(name)
            .with_context(|| format!("Initializing control {:?} value not found!", &name))?;
        animate(&objects, &name, value);
    }
    drop(controller);
    drop(edit_buffer_guard);

    Ok(InitializedInterface {
        handler,
        edit_buffer,
        dump,
        callbacks,
        widget,
        objects
    })
}

pub fn init_module_controls(config: &Config, edit_buffer: &EditBuffer) -> Result<()> {
    let mut controller = edit_buffer.controller_locked();

    for name in &config.init_controls {
        let value = controller.get(name)
            .with_context(|| format!("Initializing control {:?} value not found!", &name))?;
        controller.set_full(name, value, MIDI, Signal::Force);
    }

    Ok(())
}
