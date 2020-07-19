use anyhow::*;
use gtk::prelude::*;
use glib::Object;
use gtk::Builder;
use std::iter::Iterator;

pub struct ObjectList {
    objects: Vec<Object>
}

impl ObjectList {
    pub fn new(builder: &Builder) -> Self {
        ObjectList { objects: builder.get_objects() }
    }

    pub fn obj_by_name(&self, name: &str) -> Result<Object> {
        self.objects.iter()
            .find(|o| ObjectList::object_name(o).unwrap_or("".into()) == name)
            .with_context(|| format!("Object not found by name {:?}", name))
            .map(|obj| obj.clone())
    }

    pub fn ref_by_name<T: ObjectType>(&self, name: &str) -> Result<T> {
        let obj = self.obj_by_name(name)?;
        let cast = obj.dynamic_cast_ref::<T>()
            .with_context(|| format!("Object by name {:?} is can not be cast to type {:?}", name, T::static_type()))?
            .clone();
        Ok(cast)
    }

    pub fn obj_iter(&self) -> impl Iterator<Item=(&Object, String)> {
        self.objects.iter()
            .flat_map(|obj| ObjectList::object_name(obj).map(|name| (obj, name)))
    }

    pub fn object_name(obj: &Object) -> Option<String> {
        obj.get_property("name")
            .map(|p| p.get::<String>().unwrap())
            .unwrap_or(None)
            .filter(|v| !v.is_empty())
    }

}