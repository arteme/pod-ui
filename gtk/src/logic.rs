use std::rc::Rc;
use std::sync::{Arc, Mutex};
use pod_core::controller::Controller;
use pod_core::store::Store;
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
    origin: u8
}

impl <'c> LogicOnBuilder<'c> {
    pub fn new(builder: &'c mut LogicBuilder<'c>, name: &str) -> Self {
        Self { builder, name: name.into(), origin: 0 }
    }

    pub fn from(&mut self, origin: u8) -> &mut Self {
        self.origin = origin;
        self
    }

    // TODO: can we compose multiple `f` calls into the same callback
    //       so that controller is locked only once (and value extracted)
    //       for all `f` calls?
    pub fn run<F>(&mut self, f: F) -> &mut Self
        where F: Fn(u16, &mut Controller, u8) -> () + 'static {

        let controller = self.builder.controller.clone();
        let name = self.name.clone();
        let origin_filter = self.origin;
        self.builder.callbacks.insert(
            name.clone(),
            Rc::new(move || {
                let mut controller = controller.lock().unwrap();
                let (v, origin) = controller.get_origin(&name).unwrap();
                if origin_filter == 0 || origin == origin_filter {
                    f(v, &mut controller, origin);
                }
            })
        );

        self
    }

    pub fn on(&mut self, name: &str) -> &mut Self {
        self.name = name.into();
        self.origin = 0;
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
    origin: u8
}

impl <'c, T: Clone + 'static> LogicWithDataOnBuilder<'c, T> {
    pub fn new(builder: &'c mut LogicWithDataBuilder<'c, T>, name: &str) -> Self {
        Self { builder, name: name.into(), origin: 0 }
    }

    pub fn from(&mut self, origin: u8) -> &mut Self {
        self.origin = origin;
        self
    }

    pub fn run<F>(&mut self, f: F) -> &mut Self
        where F: Fn(u16, &mut Controller, u8, &T) -> () + 'static {

        let controller = self.builder.builder.controller.clone();
        let name = self.name.clone();
        let data = self.builder.data.clone();
        let origin_filter = self.origin;
        self.builder.builder.callbacks.insert(
            name.clone(),
            Rc::new(move || {
                let mut controller = controller.lock().unwrap();
                let (v, origin) = controller.get_origin(&name).unwrap();
                if origin_filter == 0 || origin == origin_filter {
                    f(v, &mut controller, origin, &data);
                }
            })
        );

        self
    }

    pub fn on(&mut self, name: &str) -> &mut Self {
        self.name = name.into();
        self.origin = 0;
        self
    }
}
