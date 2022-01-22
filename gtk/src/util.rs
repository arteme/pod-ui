use std::iter::repeat;
use gtk::prelude::*;
use anyhow::*;
use log::*;

use pod_core::controller::Controller;
use pod_core::store::Store;
use crate::ObjectList;

pub fn init_combo<T, F>(controller: &Controller, objs: &ObjectList,
                        name: &str, list: &Vec<T>, get_name: F) -> Result<()>
    where F: Fn(&T) -> &str
{
    let select = objs.ref_by_name::<gtk::ComboBoxText>(name)?;
    for item in list.iter() {
        let name = get_name(item);
        select.append_text(name);
    }

    let v = controller.get(name).unwrap();
    select.set_active(Some(v as u32));

    Ok(())
}

pub fn animate(objs: &ObjectList, control_name: &str, control_value: u16) {
    let prefix1 = format!("{}=", control_name);
    let prefix2 = format!("{}:", control_value);
    let catchall = "*:";
    //debug!(target: "animate", "Animate: {:?}?", control_name);
    objs.widgets_by_class_match(&|class_name| class_name.starts_with(prefix1.as_str()))
        .flat_map(|(widget, classes)| {
            let get_classes = |suffix: &str| {
                let full_len = prefix1.len() + suffix.len();
                classes.iter()
                    .filter(|c| &c[prefix1.len()..full_len] == suffix)
                    .map(|c| c[full_len..].to_string()).collect::<Vec<_>>()
            };

            let mut c = get_classes(&prefix2);
            if c.is_empty() {
                c = get_classes(catchall);
            }
            //debug!(target: "animate", "Animate: {:?} for {:?}", c, widget);

            repeat(widget.clone()).zip(c)
        })
        .for_each(|(widget, cls)| {
            match cls.as_str() {
                "show" => widget.show(),
                "hide" => widget.hide(),
                "opacity=0" => { widget.set_opacity(0f64); widget.set_sensitive(false) },
                "opacity=1" => { widget.set_opacity(1f64); widget.set_sensitive(true) },
                "enable" => widget.set_sensitive(true),
                "disable" => widget.set_sensitive(false),
                _ => {
                    warn!("Unknown animation command {:?} for widget {:?}", cls, widget)
                }
            }
        });
}
