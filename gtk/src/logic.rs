use std::rc::Rc;
use std::sync::{Arc, Mutex};
use pod_core::controller::*;
use pod_core::store::Origin;
use crate::{Callbacks, ObjectList};

pub struct LogicBuilder<'c> {
    controller: Arc<Mutex<Controller>>,
    objs: ObjectList,
    callbacks: &'c mut Callbacks
}

impl <'c> LogicBuilder<'c> {
    pub fn new(controller: Arc<Mutex<Controller>>, objs: ObjectList, callbacks: &'c mut Callbacks) -> Self {
        Self { controller, objs, callbacks }
    }

    pub fn on(&'c mut self, name: &str) -> LogicOnBuilder<'c> {
        LogicOnBuilder::new(self, name)
    }

    pub fn data<T: Clone + 'static>(&'c mut self, data: T) -> LogicWithDataBuilder<'c, T> {
        LogicWithDataBuilder::new(self, data)
    }
}

pub struct LogicOnBuilder<'c> {
    builder: &'c mut LogicBuilder<'c>,
    name: String,
    origin: Vec<Origin>
}

impl <'c> LogicOnBuilder<'c> {
    pub fn new(builder: &'c mut LogicBuilder<'c>, name: &str) -> Self {
        Self { builder, name: name.into(), origin: vec![] }
    }

    pub fn from(&mut self, origin: Origin) -> &mut Self {
        self.origin.push(origin);
        self
    }

    // TODO: can we compose multiple `f` calls into the same callback
    //       so that controller is locked only once (and value extracted)
    //       for all `f` calls?
    pub fn run<F>(&mut self, f: F) -> &mut Self
        where F: Fn(u16, &mut Controller, Origin) -> () + 'static {

        let controller = self.builder.controller.clone();
        let name = self.name.clone();
        let origin = self.origin.clone();
        self.builder.callbacks.insert(
            name.clone(),
            Rc::new(move || {
                let mut controller = controller.lock().unwrap();
                let (v, o) = controller.get_origin(&name).unwrap();
                if origin.is_empty() || origin.contains(&o) {
                    f(v, &mut controller, o);
                }
            })
        );

        self
    }

    pub fn on(&mut self, name: &str) -> &mut Self {
        self.name = name.into();
        self.origin = vec![];
        self
    }
}

pub struct LogicWithDataBuilder<'c, T> {
    builder: &'c mut LogicBuilder<'c>,
    data: T
}

impl <'c, T: Clone + 'static> LogicWithDataBuilder<'c, T> {
    pub fn new(builder: &'c mut LogicBuilder<'c>, data: T) -> Self {
        Self { builder, data }
    }

    pub fn on(&'c mut self, name: &str) -> LogicWithDataOnBuilder<'c, T> {
        LogicWithDataOnBuilder::new(self, name)
    }
}

pub struct LogicWithDataOnBuilder<'c, T> {
    builder: &'c mut LogicWithDataBuilder<'c, T>,
    name: String,
    origin: Origin
}

impl <'c, T: Clone + 'static> LogicWithDataOnBuilder<'c, T> {
    pub fn new(builder: &'c mut LogicWithDataBuilder<'c, T>, name: &str) -> Self {
        Self { builder, name: name.into(), origin: Origin::NONE }
    }

    pub fn from(&mut self, origin: Origin) -> &mut Self {
        self.origin = origin;
        self
    }

    pub fn run<F>(&mut self, f: F) -> &mut Self
        where F: Fn(u16, &mut Controller, Origin, &T) -> () + 'static {

        let controller = self.builder.builder.controller.clone();
        let name = self.name.clone();
        let data = self.builder.data.clone();
        let origin_filter = self.origin;
        self.builder.builder.callbacks.insert(
            name.clone(),
            Rc::new(move || {
                let mut controller = controller.lock().unwrap();
                let (v, origin) = controller.get_origin(&name).unwrap();
                if origin_filter == Origin::NONE || origin == origin_filter {
                    f(v, &mut controller, origin, &data);
                }
            })
        );

        self
    }

    pub fn on(&mut self, name: &str) -> &mut Self {
        self.name = name.into();
        self.origin = Origin::NONE;
        self
    }
}
