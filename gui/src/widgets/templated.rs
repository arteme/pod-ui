use std::collections::HashMap;
use string_template::Template;
use pod_gtk::prelude::*;
use pod_gtk::prelude::gtk::Widget;

struct TemplatePriv {
    label: Option<String>,
    tooltip: Option<String>
}

impl TemplatePriv {
    fn render_template(&self, data: &HashMap<&str, &str>) -> TemplatePriv {
        let label = self.label.as_ref().map(|str| {
            let t = Template::new(str.as_str());
            t.render(data)
        });

        let tooltip = self.tooltip.as_ref().map(|str| {
            let t = Template::new(str.as_str());
            t.render(data)
        });

        TemplatePriv { label, tooltip }
    }
}

fn set_template_priv<T: IsA<Widget>>(w: &T, t: TemplatePriv) {
    unsafe {
        w.set_data("template", t);
        w.connect_destroy(|w| drop_template_priv(w));
    }
}

fn get_template_priv<T: IsA<Widget>>(w: &T) -> Option<&TemplatePriv> {
    unsafe {
        w.data("template").map(|n| n.as_ref())
    }
}

fn get_or_new_template_priv<T: IsA<Widget> + TemplatedOps>(w: &T) -> &TemplatePriv {
    if let Some(t) = get_template_priv(w) {
        t
    } else {
        let t = TemplatedOps::new(w);
        set_template_priv(w, t);
        if let Some(t) = get_template_priv(w) {
            t
        } else {
            panic!("get_or_new_template_priv: got None after setting!")
        }
    }
}

fn drop_template_priv<T: IsA<Widget>>(w: &T) {
    unsafe {
        w.steal_data::<TemplatePriv>("template");
    }
}

// ---

trait TemplatedOps {
    fn new(&self) -> TemplatePriv;
    fn update(&self, t: &TemplatePriv);
}

// TODO: `self.label()` comes from concrete class extensions (GtkMenuItemExt,
//       ButtonExt) and `self.tooltip_text()` from WidgetExt. Label is probably
//       available as a text property, so this can be generalized as:
//       impl<T: IsA<gtk::Widget>> TemplatedOps for T

impl TemplatedOps for gtk::MenuItem {
    fn new(&self) -> TemplatePriv {
        let label = self.label().map(|s| s.to_string());
        let tooltip = self.tooltip_text().map(|s| s.to_string());
        TemplatePriv { label, tooltip }
    }

    fn update(&self, t: &TemplatePriv) {
        if let Some(s) = &t.label {
            self.set_label(s.as_str());
        }
        if let Some(s) = &t.tooltip {
            self.set_tooltip_text(Some(s.as_str()));
        }
    }
}

impl TemplatedOps for gtk::Button {
    fn new(&self) -> TemplatePriv {
        let label = self.label().map(|s| s.to_string());
        let tooltip = self.tooltip_text().map(|s| s.to_string());
        TemplatePriv { label, tooltip }
    }

    fn update(&self, t: &TemplatePriv) {
        if let Some(s) = &t.label {
            self.set_label(s.as_str());
        }
        if let Some(s) = &t.tooltip {
            self.set_tooltip_text(Some(s.as_str()));
        }
    }
}

// ---

pub trait Templated {
    fn init_template(&self);
    fn render_template(&self, data: &HashMap<&str, &str>);
}

impl<T: IsA<Widget> + TemplatedOps> Templated for T {
    fn init_template(&self) {
        // TODO: this does not re-initialize the underlying `TemplatePriv`
        //       if it was already initialized!
        get_or_new_template_priv(self);
    }

    fn render_template(&self, data: &HashMap<&str, &str>) {
        let t = get_or_new_template_priv(self);
        let r = t.render_template(data);
        self.update(&r);
    }
}