use std::cell::Cell;
use pod_gtk::prelude::subclass::*;
use once_cell::sync::{Lazy, OnceCell};

glib::wrapper! {
    pub struct ProgramButton(ObjectSubclass<ProgramButtonPriv>)
    @extends gtk::Bin, gtk::Container, gtk::Widget;
}

#[derive(Debug)]
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

        self.widgets.set(Widgets {
            program_id_label, program_name_label
        }).expect("Setting widgets failed");

        obj.add(&widget);
        obj.set_halign(gtk::Align::Fill);
        obj.set_valign(gtk::Align::Fill);

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
        let pb = self.instance();
        let ctx = pb.style_context();
        if modified {
            ctx.add_class("modified")
        } else {
            ctx.remove_class("modified")
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

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("programbutton");
    }

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
