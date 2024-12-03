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
    note: gtk::Label,
    octave: gtk::Label
}

pub struct TunerPriv {
    widgets: OnceCell<Widgets>
}

impl TunerPriv {
    fn set_note(&self, value: Option<usize>) {
        if let Some(w) = self.widgets.get() {
            let mut note_label: String = EMPTY.into();
            let mut octave_label: String = String::new();
            if let Some(v) = value {
                let note = NOTES[v % 12];
                note_label = note.into();
                let octave = v / 12 + 1;
                octave_label = format!("{}", octave);
            }
            w.note.set_text(&note_label);
            w.octave.set_text(&octave_label);
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
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        let left = gtk::Label::builder()
            .use_markup(true)
            .label("<span size='200%'>♭</span>")
            .build();
        obj.pack_start(&left, false, false, 0);

        let indicator = TuneIndicator::new();
        obj.pack_start(&indicator, false, false, 0);

        let right = gtk::Label::builder()
            .use_markup(true)
            .label("<span size='200%'>♯</span>")
            .build();
        obj.pack_start(&right, false, false, 0);

        let note = gtk::Label::builder()
            .label("")
            .width_chars(2)
            .margin_start(20)
            .build();
        obj.pack_start(&note, false, false, 0);

        let octave = gtk::Label::builder()
            .label("")
            .margin_top(5)
            .margin_end(20)
            .build();
        obj.pack_start(&octave, false, false, 0);

        obj.show_all();

        self.widgets.set(Widgets {
            indicator, note, octave
        }).expect("Setting widgets failed");

        self.set_note(None);
    }
}

impl WidgetImpl for TunerPriv {}
impl ContainerImpl for TunerPriv {}
impl BoxImpl for TunerPriv {}

impl Tuner {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

pub trait TunerExt {
    fn set_note(&self, value: Option<usize>);
    fn set_offset(&self, value: Option<f64>);
}

impl TunerExt for Tuner {
    fn set_note(&self, value: Option<usize>) {
        let p = TunerPriv::from_obj(self);
        p.set_note(value)
    }

    fn set_offset(&self, value: Option<f64>) {
        let p = TunerPriv::from_obj(self);
        p.set_offset(value)
    }
}