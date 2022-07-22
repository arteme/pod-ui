// TODO: redo this as a proper widget

use std::collections::HashMap;
use gtk::pango::{Style, Weight};
use crate::glib;
use crate::gtk;
use gtk::prelude::*;
use pod_gtk::ObjectList;

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
                    p.set_id_label(label.unwrap_or(glib::GString::from("")).as_ref());
                    p.set_name_label("");

                    button.children().iter().for_each(|w| button.remove(w));
                    button.add(&p.widget);

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
