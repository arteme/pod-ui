use pod_core::config::register_config;
use pod_core::model::Config;
use pod_gtk::Module;

static mut MODULES: Vec<Box<dyn Module>> = vec![];

pub fn register_module(module: impl Module + 'static) {
    for config in module.config().iter() {
        register_config(config);
    }

    unsafe {
        MODULES.push(Box::new(module))
    }
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