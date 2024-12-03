use std::cell::Cell;
use std::time::Duration;
use log::warn;
use maplit::hashmap;
use pod_gtk::prelude::subclass::*;
use once_cell::sync::{Lazy, OnceCell};
use pod_core::program_id_string;
use pod_gtk::prelude::glib::subclass::Signal;
use super::program_button::{ProgramButton, ProgramButtonExt};
use super::templated::Templated;

const NUM_BUTTONS_PER_PAGE: usize = 36;
const NUM_BUTTONS_DEFAULT: usize = NUM_BUTTONS_PER_PAGE;
const NUM_PAGES_DEFAULT: usize = 1;

glib::wrapper! {
    pub struct ProgramGrid(ObjectSubclass<ProgramGridPriv>)
    @extends gtk::Box, gtk::Container, gtk::Widget;
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "ProgramGridAction")]
pub enum ProgramGridAction {
    Load { program: usize },
    LoadUnmodified { program: usize },
    Store { program: usize },
    LoadDevice { program: usize },
    StoreDevice { program: usize }
}

#[derive(Clone, Debug)]
struct Widgets {
    size_group: gtk::SizeGroup,
    grid: gtk::Grid,
    buttons: Vec<gtk::RadioButton>,
    pages: Vec<gtk::Grid>,
    adj: gtk::Adjustment,
    left: Option<gtk::Button>,
    right: Option<gtk::Button>,
    right_click_menu: gtk::Menu,
}

pub struct ProgramGridPriv {
    num_buttons: Cell<usize>,
    num_pages: Cell<usize>,
    is_open: Cell<bool>,
    right_click_target: Cell<i32>,
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
        let pb = self.obj();
        let ctx = pb.style_context();
        if value {
            ctx.add_class("open")
        } else {
            ctx.remove_class("open")
        }

        self.is_open.set(value);
        if let Some(w) = self.widgets.get() {
            if value {
                // open
                for (i, p) in w.pages.iter().enumerate() {
                    w.grid.remove(p);
                    w.grid.attach(p, (i * 2) as i32, 0, 2, 1);
                    p.set_opacity(1.0);
                }
                w.left.as_ref().map(|b| b.hide());
                w.right.as_ref().map(|b| b.hide());
            } else {
                // close
                for p in w.pages.iter() {
                    w.grid.remove(p);
                    w.grid.attach(p, 0, 0, 2, 1);
                }
                w.left.as_ref().map(|b| b.show());
                w.right.as_ref().map(|b| b.show());
                // This doesn't signal adj's "value-changed", run the inner handler instead:
                //   self.show_page(w.adj.value() as usize);
                self.show_page_inner(w.adj.value() as usize);
            }

        }
    }

    fn open(&self) -> bool {
        self.is_open.get()
    }

    fn adj_value_changed(&self, adj: &gtk::Adjustment) {
        self.show_page_inner(adj.value() as usize);
    }

    fn left_button_clicked(&self) {
        if let Some(w) = self.widgets.get() {
            let v = w.adj.value() - w.adj.page_size();
            w.adj.set_value(v);
        }
    }

    fn right_button_clicked(&self) {
        if let Some(w) = self.widgets.get() {
            let v = w.adj.value() + w.adj.page_size();
            w.adj.set_value(v);
        }
    }

    fn button_position(i: usize) -> (usize, i32, i32) {
        let (a, b) = (i / NUM_BUTTONS_PER_PAGE, i % NUM_BUTTONS_PER_PAGE);
        let (c, d) = (b / 2, b % 2);

        let x = a * 2 + d;
        let y = c;

        let (p, x) = (x / 2, x % 2);

        (p, x as i32, y as i32)
    }

    fn show_page_inner(&self, page: usize) {
        if let Some(w) = self.widgets.get() {
            if !self.is_open.get() {
                for (i, p) in w.pages.iter().enumerate() {
                    if i == page {
                        // show
                        p.set_opacity(1.0);
                        // move to the top to receive the input events
                        w.grid.remove(p);
                        w.grid.attach(p, 0, 0, 2, 1);
                    } else {
                        // hide
                        p.set_opacity(0.0);
                    }
                }

                let left_sensitive = page > 0;
                let right_sensitive = page < w.pages.len() - 1;
                w.left.as_ref().map(|l| l.set_sensitive(left_sensitive));
                w.right.as_ref().map(|r| r.set_sensitive(right_sensitive));
            }
        }
    }

    fn show_page(&self, page: usize) {
        if let Some(w) = self.widgets.get() {
            w.adj.set_value(page as f64)
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
            .and_then(|w| w.buttons.get(program_idx))
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

    fn show_right_click_menu<T: IsA<gtk::Widget>>(&self, program_idx: usize, widget: &T, event: &gdk::Event, program_id: &str) {
        if let Some(w) = self.widgets.get() {
            w.right_click_menu.set_attach_widget(Some(widget));
            w.right_click_menu.show_all();

            let modified = self.program_modified(program_idx).unwrap_or(false);
            let show_if_modified = |widget:  &gtk::Widget| {
                let style_context = widget.style_context();
                let classes = style_context.list_classes();
                if classes.iter().any(|c| c == "show_if_modified") {
                    if modified { widget.show() } else { widget.hide() }
                }
            };

            let h = hashmap! { "program_id" => program_id };
            ObjectList::from_widget(&w.right_click_menu)
                .objects_by_type::<gtk::MenuItem>()
                .for_each(|item| {
                    show_if_modified(item.as_ref());
                    item.render_template(&h);
                });

            w.right_click_menu.popup_at_pointer(Some(event));
            self.right_click_target.set(program_idx as i32);
        }
    }

    fn right_click_menu_action(&self, action: &str) {
        let program = self.right_click_target.get();
        if program < 0 {
            return;
        }
        let program = program as usize;

        let action = match action {
            "load" => ProgramGridAction::Load { program },
            "load-unmodified" => ProgramGridAction::LoadUnmodified { program },
            "store" => ProgramGridAction::Store { program },
            "load-device" => ProgramGridAction::LoadDevice { program },
            "store-device" => ProgramGridAction::StoreDevice { program },
            _ => {
                warn!("Unknown right-click menu action: {}", action);
                return;
            }
        };

        self.obj().emit_by_name::<()>("action", &[&action]);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ProgramGridPriv {
    const NAME: &'static str = "ProgramGrid";
    type Type = ProgramGrid;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("programgrid");
    }

    fn new() -> Self {
        Self {
            num_buttons: Cell::new(NUM_BUTTONS_DEFAULT),
            num_pages: Cell::new(NUM_PAGES_DEFAULT),
            is_open: Cell::new(false),
            right_click_target: Cell::new(-1),
            widgets: OnceCell::new()
        }
    }
}

impl ObjectImpl for ProgramGridPriv {
    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecUInt::builder("num-buttons")
                    .nick("Number of buttons")
                    .blurb("Number of buttons")
                    .minimum(32)
                    .maximum(128)
                    .default_value(NUM_BUTTONS_DEFAULT as u32)
                    .write_only()
                    .construct_only()
                    .build(),
                glib::ParamSpecUInt::builder("num-pages")
                    .nick("Number of pages")
                    .blurb("Number of pages")
                    .minimum(1)
                    .maximum(10)
                    .default_value(NUM_PAGES_DEFAULT as u32)
                    .read_only()
                    .build(),
                glib::ParamSpecBoolean::builder("open")
                    .nick("Expanded")
                    .blurb("Expanded")
                    .default_value(false)
                    .build(),
            ]
        });
        PROPERTIES.as_ref()
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![
                Signal::builder("action")
                    .param_types([ProgramGridAction::static_type()])
                    .run_last()
                    .build()
            ]
        });
        SIGNALS.as_ref()
    }

    fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
        fn v<'a, T: FromValue<'a>>(value: &'a Value) -> T {
            value.get().expect("type conformity checked by `Object::set_property`")
        }
        match pspec.name() {
            "open" => self.set_open(v(value)),
            "num-buttons" => self.set_num_buttons(&(v::<u32>(value) as usize)),
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "open" => self.open().to_value(),
            "num-buttons" => (self.num_buttons() as u32).to_value(),
            "num-pages" => (self.num_pages() as u32).to_value(),
            _ => unimplemented!()
        }
    }

    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        let p = ProgramGridPriv::from_obj(&obj);
        let num_buttons = p.num_buttons.get();
        let num_pages = p.num_pages.get();

        let grid = gtk::Grid::builder()
            .column_homogeneous(true)
            .build();
        obj.pack_start(&grid, false,true, 0);
        obj.set_halign(gtk::Align::Fill);
        obj.set_valign(gtk::Align::Fill);

        let adj = gtk::Adjustment::new(0.0, 0.0, 4.0, 1.0, 1.0, 1.0);
        adj.connect_value_changed(glib::clone!(@weak self as p => move |adj| {
            p.adj_value_changed(adj);
        }));

        let size_group = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
        let mut pages = vec![];
        let mut buttons = vec![];
        let mut group = None;

        // generate pages
        for i in 0 .. num_pages {
            let name = format!("page:{}", i);
            let page = gtk::Grid::builder()
                .column_homogeneous(true)
                .row_homogeneous(true)
                .name(&name)
                .build();
            grid.attach(&page, 0, 0, 2, 1);
            pages.push(page);
        }

        // generate buttons
        for i in 0 .. (num_pages * NUM_BUTTONS_PER_PAGE) {
            let page = i / NUM_BUTTONS_PER_PAGE;
            let is_spacer = i >= num_buttons;
            let button = if !is_spacer {
                // real button
                let name = format!("program:{}", i);
                let pb = ProgramButton::new();
                let program_id = program_id_string(i);
                pb.set_program_id(&program_id);

                let b = gtk::RadioButton::builder()
                    .draw_indicator(false)
                    .name(&name)
                    .child(&pb)
                    .build();
                b.connect_toggled(glib::clone!(@weak self as p => move |button| {
                    if button.is_active() {
                        p.show_page(page);
                    }
                }));
                b.connect_button_press_event(glib::clone!(@weak self as p =>
                    @default-return Propagation::Proceed, move |button, event| {
                        if event.button() != 3 { return Propagation::Proceed }

                        p.show_right_click_menu(i, button, event, program_id.as_str());
                        Propagation::Stop
                   })
                );

                b
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

            // position the button within the pages
            let (p, x, y) = ProgramGridPriv::button_position(i);
            pages.get(p).map(|p| p.attach(&button, x, y, 1, 1));

            buttons.push(button);
        }

        // generate left/right buttons
        let (left, right) = if num_pages < 2 {
            // 1 page, no left/right buttons
            (None, None)
        } else {
            let left = gtk::Button::with_label("<");
            left.connect_clicked(glib::clone!(@weak self as p => move |_| {
                p.left_button_clicked();
            }));
            grid.attach(&left, 0, 1, 1, 1);

            let right = gtk::Button::with_label(">");
            right.connect_clicked(glib::clone!(@weak self as p => move |_| {
                p.right_button_clicked();
            }));
            grid.attach(&right, 1, 1, 1, 1);

            (Some(left), Some(right))
        };

        let menu_ui = gtk::Builder::from_string(include_str!("program_grid_menu.glade"));
        let menu: gtk::Menu = menu_ui.object("toplevel").unwrap();

        ObjectList::from_widget(&menu)
            .objects_by_type::<gtk::MenuItem>()
            .for_each(|item| {
                let name = item.widget_name().to_string();
                item.connect_activate(glib::clone!(@weak self as p => move |_| {
                    p.right_click_menu_action(name.as_str());
                }));
            });

        self.widgets.set(Widgets {
            size_group,
            buttons,
            pages,
            grid,
            adj: adj.clone(),
            right_click_menu: menu,
            left, right
        }).expect("Setting widgets failed");

        // need a delay before "value-change", which will rearrange the pages
        // in the grid (stack), otherwise the wrong page ends up on top
        glib::timeout_add_local_once(Duration::from_millis(10), move || {
            adj.emit_by_name::<()>("value-changed", &[]);
        });

    }
}

impl WidgetImpl for ProgramGridPriv {}
impl ContainerImpl for ProgramGridPriv {}
impl BoxImpl for ProgramGridPriv {}

impl ProgramGrid {
    pub fn new(num_buttons: usize) -> Self {
        glib::Object::builder()
            .property("num-buttons", num_buttons as u32)
            .property("homogeneous", true) // gtk::Box properties
            .property("spacing", 0)
            .build()
    }
}

pub trait ProgramGridExt {
    fn size_group(&self) -> gtk::SizeGroup;
    fn join_radio_group(&self, group: Option<&impl IsA<gtk::RadioButton>>);

    fn set_program_modified(&self, program_idx: usize, modified: bool);
    fn program_modified(&self, program_idx: usize) -> Option<bool>;

    fn set_program_name(&self, program_idx: usize, name: &str);
    fn program_name(&self, program_idx: usize) -> Option<glib::GString>;

    fn set_open(&self, is_open: bool);
    fn open(&self) -> bool;

    fn num_pages(&self) -> usize;
    fn num_buttons(&self) -> usize;

    fn connect_action<F>(&self, callback: F) -> glib::SignalHandlerId
        where F: Fn(ProgramGridAction) + Sync + Send + 'static;
}

impl ProgramGridExt for ProgramGrid {
    fn size_group(&self) -> gtk::SizeGroup {
        let p = ProgramGridPriv::from_obj(self);
        p.widgets.get().unwrap().size_group.clone()
    }

    fn join_radio_group(&self, group: Option<&impl IsA<gtk::RadioButton>>) {
        let p = ProgramGridPriv::from_obj(self);
        p.join_radio_group(group);
    }

    fn set_program_modified(&self, program_idx: usize, modified: bool) {
        let p = ProgramGridPriv::from_obj(self);
        p.set_program_modified(program_idx, modified)
    }

    fn program_modified(&self, program_idx: usize) -> Option<bool> {
        let p = ProgramGridPriv::from_obj(self);
        p.program_modified(program_idx)
    }

    fn set_program_name(&self, program_idx: usize, name: &str) {
        let p = ProgramGridPriv::from_obj(self);
        p.set_program_name(program_idx, name)
    }

    fn program_name(&self, program_idx: usize) -> Option<glib::GString> {
        let p = ProgramGridPriv::from_obj(self);
        p.program_name(program_idx)
    }

    fn set_open(&self, is_open: bool) {
        let p = ProgramGridPriv::from_obj(self);
        p.set_open(is_open)
    }

    fn open(&self) -> bool {
        let p = ProgramGridPriv::from_obj(self);
        p.open()
    }

    fn num_pages(&self) -> usize {
        let p = ProgramGridPriv::from_obj(self);
        p.num_pages()
    }

    fn num_buttons(&self) -> usize {
        let p = ProgramGridPriv::from_obj(self);
        p.num_buttons()
    }

    fn connect_action<F>(&self, callback: F) -> glib::SignalHandlerId
        where F: Fn(ProgramGridAction) + Sync + Send + 'static
    {
        self.connect("action", true, move |values| {
            let Some(action) = values.get(1).ok_or("Failed to get argument".to_string())
                .and_then(|v| v.get::<ProgramGridAction>().map_err(|e| e.to_string()))
                .map_err(|e| { warn!("Failed to get ProgramGridAction: {}", e) })
                .ok() else {
                return None
            };

            callback(action);
            None
        })
    }
}