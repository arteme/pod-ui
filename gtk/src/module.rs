use std::sync::{Arc, Mutex};
use pod_core::controller::Controller;
use pod_core::model::Config;
use anyhow::Result;
use multimap::MultiMap;
use pod_core::raw::Raw;

use crate::ObjectList;

pub type Callbacks = MultiMap<String, Box<dyn Fn() -> ()>>;

pub trait Module {
    fn config(&self) -> Config;
    fn widget(&self) -> gtk::Widget;
    fn objects(&self) -> ObjectList;

    fn wire(&self, controller: Arc<Mutex<Controller>>, raw: Arc<Mutex<Raw>>, callbacks: &mut Callbacks) -> Result<()>;
    fn init(&self, controller: Arc<Mutex<Controller>>, raw: Arc<Mutex<Raw>>) -> Result<()>;
}