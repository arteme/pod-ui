use once_cell::sync::{Lazy, OnceCell};
use pod_gtk::prelude::subclass::*;
use crate::widgets::{TuneIndicator, TuneIndicatorExt};

static NOTES: Lazy<Vec<&str>> = Lazy::new(|| {
   vec!["B", "C", "D♭", "D", "E♭", "E", "F", "G♭", "G", "A♭", "A", "B♭"]
});

static EMPTY: &str = "—";

glib::wrapper! {
    pub struct Tuner(ObjectSubclass<TunerPriv>)
    @extends gtk::Box, gtk::Container, gtk::Widget;
}

#[derive(Clone, Debug)]
struct Widgets {
    indicator: TuneIndicator,
    note: gtk::Label
}

pub struct TunerPriv {
    widgets: OnceCell<Widgets>
}

impl TunerPriv {
    fn set_note(&self, value: Option<usize>) {
        if let Some(w) = self.widgets.get() {
            let mut label: String = EMPTY.into();
            if let Some(v) = value {
                let note = NOTES[v % 12];
                let octave = v / 12 + 1;
                label = format!("{}<sub>{}</sub>", note, octave);
            }
            w.note.set_markup(&label);
        }
    }

    fn set_offset(&self, value: Option<f64>) {
        if let Some(w) = self.widgets.get() {
            w.indicator.set_pos(value);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for TunerPriv {
    const NAME: &'static str = "Tuner";
    type Type = Tuner;
    type ParentType = gtk::Box;

    fn new() -> Self {
        Self {
            widgets: OnceCell::new()
        }
    }
}

impl ObjectImpl for TunerPriv {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        let indicator = TuneIndicator::new();
        indicator.show();
        obj.pack_start(&indicator, true, true, 0);

        let note = gtk::Label::new(None);
        note.set_margin_top(9);
        note.set_width_request(10);
        note.show();
        obj.pack_start(&note, false, false, 0);

        self.widgets.set(Widgets {
            indicator, note
        }).expect("Setting widgets failed");

        self.set_note(None);
    }
}

impl WidgetImpl for TunerPriv {}
impl ContainerImpl for TunerPriv {}
impl BoxImpl for TunerPriv {}

impl Tuner {
    pub fn new() -> Self {
        glib::Object::new(&[
            ("spacing", &10) // gtk::Box
        ])
        .expect("Failed to create Tuner")
    }
}

pub trait TunerExt {
    fn set_note(&self, value: Option<usize>);
    fn set_offset(&self, value: Option<f64>);
}

impl TunerExt for Tuner {
    fn set_note(&self, value: Option<usize>) {
        let p = TunerPriv::from_instance(self);
        p.set_note(value)
    }

    fn set_offset(&self, value: Option<f64>) {
        let p = TunerPriv::from_instance(self);
        p.set_offset(value)
    }
}