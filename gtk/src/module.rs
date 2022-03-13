use std::sync::{Arc, Mutex};
use pod_core::model::Config;
use anyhow::Result;
use multimap::MultiMap;
use pod_core::edit::EditBuffer;

use crate::ObjectList;

pub type Callbacks = MultiMap<String, Box<dyn Fn() -> ()>>;

pub trait Module {
    fn config(&self) -> Box<[Config]>;
    fn widget(&self) -> gtk::Widget;
    fn objects(&self) -> ObjectList;

    fn wire(&self, config: &Config, controller: Arc<Mutex<EditBuffer>>, callbacks: &mut Callbacks) -> Result<()>;
    fn init(&self, config: &Config, controller: Arc<Mutex<EditBuffer>>) -> Result<()>;
}