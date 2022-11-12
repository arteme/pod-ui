use std::rc::Rc;
use std::sync::{Arc, Mutex};
use pod_core::model::Config;
use anyhow::Result;
use multimap::MultiMap;
use pod_core::edit::EditBuffer;
use pod_core::handler::Handler;

use crate::ObjectList;

pub type Callbacks = MultiMap<String, Rc<dyn Fn() -> ()>>;

pub trait Module {
    fn config(&self) -> Box<[Config]>;
    fn init(&self, config: &'static Config) -> Box<dyn Interface>;
    fn handler(&self, config: &'static Config) -> Box<dyn Handler + 'static + Sync + Send>;
}

pub trait Interface {
    fn widget(&self) -> gtk::Widget;
    fn objects(&self) -> ObjectList;

    fn wire(&self, edit_buffer: Arc<Mutex<EditBuffer>>, callbacks: &mut Callbacks) -> Result<()>;
    fn init(&self, edit_buffer: Arc<Mutex<EditBuffer>>) -> Result<()>;
}