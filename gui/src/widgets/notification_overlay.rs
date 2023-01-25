use std::time::Duration;
use once_cell::sync::OnceCell;
use pod_gtk::prelude::glib::SourceId;
use pod_gtk::prelude::glib::translate::from_glib;
use pod_gtk::prelude::gtk::gdk::EventMask;
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
    fn add_notification(&self, label: &str, id: &str) {
        let Some(w) = self.widgets.get() else { return };

        let rev = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .transition_duration(1000)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .build();

        // make sure these are done before rev is added to the notification box
        rev.set_has_window(true);
        rev.set_sensitive(true);
        rev.set_events(EventMask::BUTTON_PRESS_MASK | EventMask::BUTTON_RELEASE_MASK);

        let label = gtk::Label::builder()
            .margin_end(2)
            .use_markup(true)
            .label(label)
            .build();

        let sc = label.style_context();
        sc.add_class("app-notification");
        if id.len() > 0 {
            let class_name = format!("id={}", id);
            sc.add_class(&class_name);
        }
        rev.add(&label);
        rev.set_has_window(true);
        rev.show_all();

        w.notifications_box.add(&rev);

        // show the notification
        rev.set_reveal_child(true);

        // remove notification when hiding animation is complete
        rev.connect_child_revealed_notify(
            glib::clone!(@weak w.notifications_box as n => @default-return (), move |rev| {
                if !rev.is_child_revealed() {
                    n.remove(rev);
                }
        }));

        // dismiss notification on click on the revealer
        rev.connect_button_release_event(|rev, _| {
            if rev.reveals_child() {
                rev.set_reveal_child(false);
            }
            Inhibit(false)
        });

        // dismiss notification after 5 seconds of showing it
        Self::add_notification_timeout(&label, &rev);
    }

    fn add_notification_timeout(label: &gtk::Label, rev: &gtk::Revealer) {
        let source_id: Option<SourceId> = unsafe {
            label.steal_data::<u32>("timeout-id").map(|v| from_glib(v))
        };
        if let Some(source_id) = source_id {
            // TODO: this is nasty, since `SourceId::remove()` will panic if the
            //       source does not exist. Ok for now, since we always steal the
            //       data, but gotta figure out a less error-prone method to pass
            //       the source id around.
            source_id.remove();
        }

        if !rev.reveals_child() {
            rev.set_reveal_child(true);
        }

        // dismiss notification after 5 seconds of showing it
        let source_id = glib::timeout_add_local_once(
            Duration::from_millis(5000),
            glib::clone!(@weak rev, @weak label => @default-return (), move || {
                unsafe { label.steal_data::<u32>("timeout-id") };
                if rev.reveals_child() {
                    rev.set_reveal_child(false);
                }
            })
        );

        unsafe {
            let source_id = source_id.as_raw();
            label.set_data("timeout-id", source_id);
        }
    }

    fn update_notification(&self, label: &str, id: &str) -> bool {
        let Some(w) = self.widgets.get() else { return false };

        let objs = ObjectList::from_widget(&w.overlay);
        let class_name = format!("id={}", id);

        let Some((widget, _)) =
            objs.widgets_by_class_match(|c| c == &class_name).next() else {
            return false;
        };

        let Some(widget) = widget.dynamic_cast_ref::<gtk::Label>() else { return false; };
        let Some(rev) = widget.parent()
            .and_then(|w| w.dynamic_cast::<gtk::Revealer>().ok()) else { return false; };
        widget.set_label(label);

        // reset the dismiss timeout
        Self::add_notification_timeout(widget, &rev);

        true
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
        notifications_box.set_valign(gtk::Align::Start); // not needed, strictly speaking
        notifications_box.set_margin_top(2);
        overlay.add_overlay(&notifications_box);
        overlay.set_overlay_pass_through(&notifications_box, true);

        self.widgets.set(Widgets {
            overlay,
            notifications_box
        }).expect("Setting widgets failed");
    }
}

impl WidgetImpl for NotificationOverlayPriv {}
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
    fn add_notification_with_id(&self, label: &str, id: &str);
    fn update_notification_with_id(&self, label: &str, id: &str) -> bool;
}

impl NotificationOverlayExt for NotificationOverlay {
    fn add_notification(&self, label: &str) {
        let p = NotificationOverlayPriv::from_instance(self);
        p.add_notification(label, &"");
    }

    fn add_notification_with_id(&self, label: &str, id: &str) {
        let p = NotificationOverlayPriv::from_instance(self);
        p.add_notification(label, id);
    }

    fn update_notification_with_id(&self, label: &str, id: &str) -> bool {
        let p = NotificationOverlayPriv::from_instance(self);
        p.update_notification(label, id)
    }
}