use std::cell::Cell;
use crate::{glib, ProgramButtonExt};
use crate::gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use log::debug;
use once_cell::sync::{Lazy, OnceCell};
use crate::glib::{ParamSpec, Value};
use crate::glib::value::FromValue;
use crate::program_button::ProgramButton;

const NUM_BUTTONS_PER_PAGE: usize = 36;
const NUM_BUTTONS_DEFAULT: usize = NUM_BUTTONS_PER_PAGE;
const NUM_PAGES_DEFAULT: usize = 1;

glib::wrapper! {
    pub struct ProgramGrid(ObjectSubclass<ProgramGridPriv>)
    @extends gtk::Box, gtk::Container, gtk::Widget;
}

#[derive(Clone, Debug)]
struct Widgets {
    size_group: gtk::SizeGroup,
    grid: gtk::Grid,
    buttons: Vec<gtk::RadioButton>,
    adj: gtk::Adjustment,
    left: Option<gtk::Button>,
    right: Option<gtk::Button>,
}

pub struct ProgramGridPriv {
    num_buttons: Cell<usize>,
    num_pages: Cell<usize>,
    is_open: Cell<bool>,
    widgets: OnceCell<Widgets>
}



impl ProgramGridPriv {
    fn set_num_buttons(&self, value: &usize) {
        self.num_buttons.set(*value);
        self.num_pages.set((*value + NUM_BUTTONS_PER_PAGE - 1) / NUM_BUTTONS_PER_PAGE);
    }

    fn num_buttons(&self) -> usize {
        self.num_buttons.get()
    }

    fn num_pages(&self) -> usize {
        self.num_pages.get()
    }

    fn set_open(&self, value: bool) {
        self.is_open.set(value);
        let page = if value {
            // expanded view
            -1
        } else {
            if let Some(w) = self.widgets.get() {
                w.adj.value() as i32
            } else {
                0
            }
        };
        self.show_page(page);
    }

    fn open(&self) -> bool {
        self.is_open.get()
    }

    fn adj_value_changed(&self, adj: &gtk::Adjustment) {
        debug!("adj_value_changed");
        let value = adj.value();
        let page_size = adj.page_size();
        let upper = adj.upper();

        debug!("value={}", value);

        if let Some(w) = self.widgets.get() {
            w.left.as_ref().map(|l| l.set_sensitive(value > 0.0) );
            w.right.as_ref().map(|r| r.set_sensitive(value < upper - page_size) );
            self.show_page(value as i32);
        }
    }

    fn left_button_clicked(&self) {
        debug!("left_button_clicked");
        if let Some(w) = self.widgets.get() {
            let v = w.adj.value();
            let v = f64::max(0.0, v - w.adj.page_size());
            w.adj.set_value(v);
        }
    }

    fn right_button_clicked(&self) {
        debug!("right_button_clicked");
        if let Some(w) = self.widgets.get() {
            let v = w.adj.value();
            let v = f64::min(w.adj.upper(), v + w.adj.page_size());
            w.adj.set_value(v);
        }
    }

    fn show_page(&self, page: i32) {
        if let Some(w) = self.widgets.get() {
            for (i, button) in w.buttons.iter().enumerate() {
                let (a, b) = (i / NUM_BUTTONS_PER_PAGE, i % NUM_BUTTONS_PER_PAGE);
                let (c, d) = (b / 2, b % 2);

                let mut x = (a * 2 + d) as i32;
                let y = c as i32;

                w.grid.remove(button);

                if page != -1 {
                    let l = page * 2;
                    let h = l + 1;
                    if x < l || x > h {
                        continue;
                    }
                    x -= l;
                }

                w.grid.attach(button, x, y, 1, 1);
                button.show_all();
            }
        }

    }

    fn join_radio_group(&self, group: Option<&impl IsA<gtk::RadioButton>>) {
        if let Some(w) = self.widgets.get() {
            for b in w.buttons.iter() {
                b.join_group(group);
            }
        }
    }

    fn program_button(&self, program_idx: usize) -> Option<ProgramButton> {
        self.widgets.get()
            .and_then(|w| w.buttons.get(program_idx - 1))
            .and_then(|b| b.child())
            .and_then(|w| w.dynamic_cast::<ProgramButton>().ok())
    }

    fn set_program_modified(&self, program_idx: usize, modified: bool) {
        self.program_button(program_idx)
            .map(|p| p.set_modified(modified));
    }

    fn program_modified(&self, program_idx: usize) -> Option<bool> {
        self.program_button(program_idx)
            .map(|p| p.modified())
    }

    fn set_program_name(&self, program_idx: usize, name: &str) {
        self.program_button(program_idx)
            .map(|p| p.set_program_name(name));
    }

    fn program_name(&self, program_idx: usize) -> Option<glib::GString> {
        self.program_button(program_idx)
            .map(|p| p.program_name())
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ProgramGridPriv {
    const NAME: &'static str = "ProgramGrid";
    type Type = ProgramGrid;
    type ParentType = gtk::Box;

    fn new() -> Self {
        Self {
            num_buttons: Cell::new(NUM_BUTTONS_DEFAULT),
            num_pages: Cell::new(NUM_PAGES_DEFAULT),
            is_open: Cell::new(false),
            widgets: OnceCell::new()
        }
    }
}

impl ObjectImpl for ProgramGridPriv {
    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecUInt::new(
                    "num-buttons",
                    "Number of buttons",
                    "Number of buttons",
                    32,
                    124,
                    NUM_BUTTONS_DEFAULT as u32,
                    glib::ParamFlags::WRITABLE | glib::ParamFlags::CONSTRUCT_ONLY
                ),
                glib::ParamSpecUInt::new(
                    "num-pages",
                    "Number of pages",
                    "Number of pages",
                    1,
                    10,
                    NUM_PAGES_DEFAULT as u32,
                    glib::ParamFlags::READABLE
                ),
                glib::ParamSpecBoolean::new(
                    "open",
                    "Expanded",
                    "Expanded",
                    false,
                    glib::ParamFlags::READWRITE
                ),
            ]
        });
        PROPERTIES.as_ref()
    }

//    fn signals() -> &'static [Signal] {
//        todo!()
//    }

    fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
        fn v<'a, T: FromValue<'a>>(value: &'a Value) -> T {
            value.get().expect("type conformity checked by `Object::set_property`")
        }
        match pspec.name() {
            "open" => self.set_open(v(value)),
            "num-buttons" => self.set_num_buttons(&(v::<u32>(value) as usize)),
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "open" => self.open().to_value(),
            "num-buttons" => (self.num_buttons() as u32).to_value(),
            "num-pages" => (self.num_pages() as u32).to_value(),
            _ => unimplemented!()
        }
    }

    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        let p = ProgramGridPriv::from_instance(obj);
        let num_buttons = p.num_buttons.get() as i32;
        let num_pages = p.num_pages.get() as i32;

        let grid = gtk::Grid::builder()
            .build();
        obj.pack_start(&grid, false,true, 0);
        obj.set_halign(gtk::Align::Fill);
        obj.set_valign(gtk::Align::Fill);

        let adj = gtk::Adjustment::new(0.0, 0.0, 4.0, 1.0, 1.0, 1.0);
        adj.connect_value_changed(glib::clone!(@weak obj => move |adj| {
            let p = ProgramGridPriv::from_instance(&obj);
            p.adj_value_changed(adj);
        }));

        /*grid.connect_size_allocate(move |w, s| {
            println!("grid size-allocate! {:?}", s);
        });*/

        let size_group = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
        let mut buttons = vec![];
        let mut group = None;

        for i in 0 .. (num_pages * NUM_BUTTONS_PER_PAGE as i32) {
            let is_spacer = i >= num_buttons;
            let button = if !is_spacer {
                // real button
                let (a, b) = (i / 4, i % 4);

                let name = format!("program:{}", i + 1);
                let pb = ProgramButton::new();
                let program_id = format!("{}{}", a + 1, char::from_u32('A' as u32 + b as u32).unwrap());
                pb.set_program_id(&program_id);

                gtk::RadioButton::builder()
                    .draw_indicator(false)
                    .name(&name)
                    .child(&pb)
                    .build()
            } else {
                // spacer
                gtk::RadioButton::builder()
                    .draw_indicator(false)
                    .sensitive(false)
                    .relief(gtk::ReliefStyle::None)
                    .build()
            };

            if group.is_some() {
                button.join_group(group.as_ref());
            } else {
                group = Some(button.clone());
            }

            button.set_hexpand(true);

            size_group.add_widget(&button);
            buttons.push(button);
        }

        let (left, right) = if num_pages < 2 {
            // 1 page, no left/right buttons
            (None, None)
        } else {
            let top = NUM_BUTTONS_PER_PAGE as i32 / 2;
            let left = gtk::Button::with_label("<");
            left.connect_clicked(glib::clone!(@weak obj => move |_| {
                let p = ProgramGridPriv::from_instance(&obj);
                p.left_button_clicked();
            }));
            grid.attach(&left, 0, top, 1, 1);

            let right = gtk::Button::with_label(">");
            right.connect_clicked(glib::clone!(@weak obj => move |_| {
                let p = ProgramGridPriv::from_instance(&obj);
                p.right_button_clicked();
            }));
            grid.attach(&right, 1, top, 1, 1);

            (Some(left), Some(right))
        };


        self.widgets.set(Widgets {
            size_group,
            buttons,
            grid,
            adj: adj.clone(),
            left, right
        }).expect("Setting widgets failed");

        adj.emit_by_name::<()>("value-changed", &[]);
    }
}

impl WidgetImpl for ProgramGridPriv {}
impl ContainerImpl for ProgramGridPriv {}
impl BoxImpl for ProgramGridPriv {}

impl ProgramGrid {
    pub fn new(num_buttons: usize) -> Self {

        glib::Object::new(&[
            ("num-buttons", &(num_buttons as u32)),
            ("homogeneous", &true), // gtk::Box properties
            ("spacing", &0)
        ])
        .expect("Failed to create ProgramGrid")
    }
}

pub trait ProgramGridExt {
    fn size_group(&self) -> gtk::SizeGroup;
    fn join_radio_group(&self, group: Option<&impl IsA<gtk::RadioButton>>);

    fn set_program_modified(&self, program_idx: usize, modified: bool);
    fn program_modified(&self, program_idx: usize) -> Option<bool>;

    fn set_program_name(&self, program_idx: usize, name: &str);
    fn program_name(&self, program_idx: usize) -> Option<glib::GString>;
}

impl ProgramGridExt for ProgramGrid {
    fn size_group(&self) -> gtk::SizeGroup {
        let p = ProgramGridPriv::from_instance(self);
        p.widgets.get().unwrap().size_group.clone()
    }

    fn join_radio_group(&self, group: Option<&impl IsA<gtk::RadioButton>>) {
        let p = ProgramGridPriv::from_instance(self);
        p.join_radio_group(group);
    }

    fn set_program_modified(&self, program_idx: usize, modified: bool) {
        let p = ProgramGridPriv::from_instance(self);
        p.set_program_modified(program_idx, modified)
    }

    fn program_modified(&self, program_idx: usize) -> Option<bool> {
        let p = ProgramGridPriv::from_instance(self);
        p.program_modified(program_idx)
    }

    fn set_program_name(&self, program_idx: usize, name: &str) {
        let p = ProgramGridPriv::from_instance(self);
        p.set_program_name(program_idx, name)
    }

    fn program_name(&self, program_idx: usize) -> Option<glib::GString> {
        let p = ProgramGridPriv::from_instance(self);
        p.program_name(program_idx)
    }
}