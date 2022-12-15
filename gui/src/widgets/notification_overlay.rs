use std::time::Duration;
use once_cell::sync::OnceCell;
use pod_gtk::prelude::gtk::gdk::{EventButton, EventMask};
use pod_gtk::prelude::gtk::Widget;
use pod_gtk::prelude::subclass::*;

glib::wrapper! {
    pub struct NotificationOverlay(ObjectSubclass<NotificationOverlayPriv>)
    @extends gtk::Bin, gtk::Container, gtk::Widget;
}

#[derive(Debug)]
struct Widgets {
    overlay: gtk::Overlay,
    notifications_box: gtk::Box,
}

pub struct NotificationOverlayPriv {
    widgets: OnceCell<Widgets>
}

impl NotificationOverlayPriv {
    fn add_notification(&self, label: &str) {
        let Some(w) = self.widgets.get() else { return };

        let rev = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .transition_duration(1000)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .build();

        let label = gtk::Label::builder()
            .margin_end(2)
            .use_markup(true)
            .label(label)
            .build();

        let sc = label.style_context();
        sc.add_class("app-notification");
        rev.add(&label);
        rev.show_all();

        w.notifications_box.add(&rev);

        rev.set_reveal_child(true);

        rev.connect_child_revealed_notify(
            glib::clone!(@weak w.notifications_box as n => @default-return (), move |rev| {
                if !rev.is_child_revealed() {
                    n.remove(rev);
                }
        }));

        label.set_has_window(true);
        label.set_sensitive(true);
        label.set_events(EventMask::ALL_EVENTS_MASK);
        label.connect_button_release_event(
            glib::clone!(@weak rev => @default-return Inhibit(false), move |_, event| {
                if !event.is::<EventButton>() { return Inhibit(false) }
                if rev.reveals_child() {
                    rev.set_reveal_child(false);
                }
                Inhibit(false)
            })
        );

        glib::timeout_add_local_once(
            Duration::from_millis(5000),
            glib::clone!(@weak rev => @default-return (), move || {
                if rev.reveals_child() {
                    rev.set_reveal_child(false);
                }
            })
        );
    }
}

#[glib::object_subclass]
impl ObjectSubclass for NotificationOverlayPriv {
    const NAME: &'static str = "NotificationOverlay";
    type Type = NotificationOverlay;
    type ParentType = gtk::Bin;

    fn new() -> Self {
        Self {
            widgets: OnceCell::new()
        }
    }
}

impl ObjectImpl for NotificationOverlayPriv {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        let overlay = gtk::Overlay::new();
        self.parent_add(&obj, &overlay.clone().upcast());

        let notifications_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        notifications_box.set_margin_top(2);
        overlay.add_overlay(&notifications_box);

        self.widgets.set(Widgets {
            overlay,
            notifications_box
        }).expect("Setting widgets failed");
    }
}

impl WidgetImpl for NotificationOverlayPriv {

}
impl ContainerImpl for NotificationOverlayPriv {
    fn add(&self, _container: &Self::Type, widget: &Widget) {
        let Some(w) = self.widgets.get() else { return; };
        w.overlay.add(widget);
    }

    fn remove(&self, _container: &Self::Type, widget: &Widget) {
        let Some(w) = self.widgets.get() else { return; };
        w.overlay.remove(widget);
    }
}
impl BinImpl for NotificationOverlayPriv {}

impl NotificationOverlay {
    pub fn new() -> Self {
        glib::Object::new(&[])
            .expect("Failed to create NotificationOverlay")
    }
}

pub trait NotificationOverlayExt {
    fn add_notification(&self, label: &str);
}

impl NotificationOverlayExt for NotificationOverlay {
    fn add_notification(&self, label: &str) {
        let p = NotificationOverlayPriv::from_instance(self);
        p.add_notification(label);
    }
}