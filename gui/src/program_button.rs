use std::cell::Cell;
use std::collections::HashMap;
use std::ptr::NonNull;
use gtk::pango::{Style, Weight};
use crate::glib;
use crate::gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::{Lazy, OnceCell};
use pod_gtk::ObjectList;
use crate::glib::subclass::TypeData;
use crate::glib::{ParamSpec, Type, Value};
use crate::glib::value::FromValue;
use crate::gtk::Align;

glib::wrapper! {
    pub struct ProgramButton(ObjectSubclass<ProgramButtonPriv>)
    @extends gtk::Bin, gtk::Container, gtk::Widget;
}

struct Widgets {
    program_id_label: gtk::Label,
    program_name_label: gtk::Label
}

pub struct ProgramButtonPriv {
    widgets: OnceCell<Widgets>,
    modified: Cell<bool>
}

impl ProgramButtonPriv {
    fn init(&self, obj: &ProgramButton) {
        let ui = gtk::Builder::from_string(include_str!("program_button.glade"));
        let widget: gtk::Widget = ui.objects()[0].clone().dynamic_cast::<gtk::Widget>().unwrap();
        let program_id_label: gtk::Label = ui.object("program_id_label").unwrap();
        let program_name_label: gtk::Label = ui.object("program_name_label").unwrap();

        if self.widgets.set(Widgets {
            program_id_label, program_name_label
        }).is_err() {
            // TODO
        }

        obj.add(&widget);
        obj.set_halign(Align::Fill);
        obj.set_valign(Align::Fill);

        self.set_program_id("");
        self.set_program_name("");
    }

    fn set_program_id(&self, value: &str) {
        if let Some(w) = self.widgets.get() {
            w.program_id_label.set_label(value)
        }
    }

    fn program_id(&self) -> glib::GString {
        if let Some(w) = self.widgets.get() {
            return w.program_id_label.label()
        }
        return "".into()
    }

    fn set_program_name(&self, value: &str) {
        if let Some(w) = self.widgets.get() {
            w.program_name_label.set_label(value)
        }
    }

    fn program_name(&self) -> glib::GString {
        if let Some(w) = self.widgets.get() {
            return w.program_name_label.label()
        }
        return "".into()
    }

    fn set_modified(&self, modified: bool) {
        let set_label_font = |label: &gtk::Label| {
            let pc = label.pango_context();
            let mut fd = pc.font_description().unwrap();

            if self.modified.get() == modified {
                return;
            }

            if modified {
                fd.set_weight(Weight::Bold);
                fd.set_style(Style::Italic);
            } else {
                fd.set_weight(Weight::Normal);
                fd.set_style(Style::Normal);
            }
            pc.set_font_description(&fd);
            label.queue_draw();
            label.queue_resize();
        };

        if let Some(w) = self.widgets.get() {
            set_label_font(&w.program_id_label);
            set_label_font(&w.program_name_label);
        }

        self.modified.set(modified);
    }

    fn modified(&self) -> bool {
        return self.modified.get()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ProgramButtonPriv {
    const NAME: &'static str = "ProgramButton";
    type Type = ProgramButton;
    type ParentType = gtk::Bin;

    fn new() -> Self {
        Self {
            widgets: OnceCell::new(),
            modified: Cell::new(false)
        }
    }
}

impl ObjectImpl for ProgramButtonPriv {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);
        self.init(obj);
    }

    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
           vec![
               glib::ParamSpecString::new(
                   "program-id",
                   "Program Id",
                   "The id of the program",
                   None,
                   glib::ParamFlags::READWRITE
               ),
               glib::ParamSpecString::new(
                   "program-name",
                   "Program Name",
                   "The name of the program",
                   None,
                   glib::ParamFlags::READWRITE
               ),
               glib::ParamSpecBoolean::new(
                   "modified",
                   "Modified",
                   "The modified flag",
                   false,
                   glib::ParamFlags::READWRITE
               ),
           ]
        });
        PROPERTIES.as_ref()
    }

    fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
        fn v<'a, T: FromValue<'a>>(value: &'a Value) -> T {
            value.get().expect("type conformity checked by `Object::set_property`")
        }
        match pspec.name() {
            "program-id" => self.set_program_id(v(value)),
            "program-name" => self.set_program_name(v(value)),
            "modified" => self.set_modified(v(value)),
            _ => unimplemented!()
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "program-id" => self.program_id().to_value(),
            "program-name" => self.program_name().to_value(),
            "modified" => self.modified().to_value(),
            _ => unimplemented!()
        }
    }
}

impl WidgetImpl for ProgramButtonPriv {}
impl ContainerImpl for ProgramButtonPriv {}
impl BinImpl for ProgramButtonPriv {}

impl ProgramButton {
    pub fn new() -> Self {
        glib::Object::new(&[])
            .expect("Failed to create ProgramButton")
    }
}

pub trait ProgramButtonExt {
    fn set_program_id(&self, value: &str);
    fn program_id(&self) -> glib::GString;

    fn set_program_name(&self, value: &str);
    fn program_name(&self) -> glib::GString;

    fn set_modified(&self, modified: bool);
    fn modified(&self) -> bool;
}

impl ProgramButtonExt for ProgramButton {
    fn set_program_id(&self, value: &str) {
        let p = ProgramButtonPriv::from_instance(self);
        p.set_program_id(value);
    }

    fn program_id(&self) -> glib::GString {
        let p = ProgramButtonPriv::from_instance(self);
        p.program_id()
    }

    fn set_program_name(&self, value: &str) {
        let p = ProgramButtonPriv::from_instance(self);
        p.set_program_name(value)
    }

    fn program_name(&self) -> glib::GString {
        let p = ProgramButtonPriv::from_instance(self);
        p.program_name()
    }

    fn set_modified(&self, modified: bool) {
        let p = ProgramButtonPriv::from_instance(self);
        p.set_modified(modified)
    }

    fn modified(&self) -> bool {
        let p = ProgramButtonPriv::from_instance(self);
        p.modified()
    }
}

/*
pub struct ProgramButton {
    pub widget: gtk::Widget,
    program_id_label: gtk::Label,
    program_name_label: gtk::Label,
    pub modified: bool
}

impl ProgramButton {
    fn new() -> Self {
        let ui = gtk::Builder::from_string(include_str!("program_button.glade"));
        let widget: gtk::Widget = ui.objects()[0].clone().dynamic_cast::<gtk::Widget>().unwrap();
        let program_id_label: gtk::Label = ui.object("program_id_label").unwrap();
        let program_name_label: gtk::Label = ui.object("program_name_label").unwrap();

        ProgramButton { widget, program_id_label, program_name_label, modified: false }
    }

    pub fn set_id_label(&self, label: &str) {
        self.program_id_label.set_label(label);
    }

    pub fn set_name_label(&self, label: &str) {
        self.program_name_label.set_label(label);
    }

    pub fn set_modified(&mut self, modified: bool) {
        let set_label_font = |label: &gtk::Label| {
            let pc = label.pango_context();
            let mut fd = pc.font_description().unwrap();

            if self.modified == modified {
                return;
            }

            if modified {
                fd.set_weight(Weight::Bold);
                fd.set_style(Style::Italic);
            } else {
                fd.set_weight(Weight::Normal);
                fd.set_style(Style::Normal);
            }
            pc.set_font_description(&fd);
            label.queue_draw();
        };

        set_label_font(&self.program_id_label);
        set_label_font(&self.program_name_label);
        self.modified = modified;
    }
}
 */

pub struct ProgramButtons {
    buttons: HashMap<String, ProgramButton>
}

impl ProgramButtons {
    pub fn new(objects: &ObjectList) -> Self {
        let buttons: HashMap<String, ProgramButton> =
            objects.named_objects()
                .filter(|(obj, name)| {
                    let classes =
                        obj.dynamic_cast_ref::<gtk::Widget>().unwrap().style_context().list_classes();
                    name.starts_with("program:") && !classes.iter().any(|n| n.as_str() == "no_program_name")
                }
                )
                .map(|(obj, name)| {
                    let button = obj.dynamic_cast_ref::<gtk::RadioButton>().unwrap();
                    let label = button.label();

                    let p = ProgramButton::new();
                    p.set_program_id(label.unwrap_or(glib::GString::from("")).as_ref());
                    p.set_program_name("");

                    button.children().iter().for_each(|w| button.remove(w));
                    button.add(&p);

                    (name, p)
                })
                .collect();

        ProgramButtons { buttons }
    }

    pub fn get(&self, patch: usize) -> Option<&ProgramButton> {
        self.buttons.get(&format!("program:{}", patch))
    }

    pub fn get_mut(&mut self, patch: usize) -> Option<&mut ProgramButton> {
        self.buttons.get_mut(&format!("program:{}", patch))
    }

    pub fn set_modified(&mut self, idx: usize, modified: bool) {
        self.get_mut(idx).map(|button| button.set_modified(modified));
    }

    pub fn set_all_modified(&mut self, modified: bool) {
        self.buttons.iter_mut()
            .for_each(|(_, button)| button.set_modified(modified));
    }
}
