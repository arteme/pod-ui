use std::rc::Rc;
use std::sync::{Arc, Mutex};
use anyhow::*;
use pod_core::controller::*;
use pod_core::model::Toggle;
use pod_core::store::Store;
use gtk::prelude::*;
use log::warn;
use crate::{Callbacks, ObjectList};

pub fn wire_toggles(container_name: &str,
                    toggles: &Vec<Toggle>,
                    controller: Arc<Mutex<Controller>>,
                    objs: &ObjectList,
                    callbacks: &mut Callbacks) -> Result<()> {
    let grid = objs.ref_by_name::<gtk::Grid>(container_name)?;
    // The grid is intended to have homogeneous column size, but if we rely on GTK
    // grid "column-homogeneous" property, then the total grid changes minimum size
    // as widgets below are repositioned. Seems like a bug (arch linux gtk-1:3.24.35-1)
    //
    // Instead, we'll set grid's "column-homogeneous" to false, its children's
    // "hexpand" to true and add a `gtk::SizeGroup` to make the widths homogeneous.
    grid.set_column_homogeneous(false);
    let sg = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);

    for toggle in toggles.iter() {
        let widget = objs.ref_by_name::<gtk::Widget>(&toggle.name)?;
        widget.set_hexpand(true);
        sg.add_widget(&widget);
        if let Some(parent) = widget.parent() {
            let parent = parent.dynamic_cast_ref::<gtk::Container>().unwrap();
            parent.remove(&widget);
        }
        grid.attach(&widget, toggle.off_position as i32, 1, 1, 1);
        if toggle.position_control.is_empty() { continue; }

        // add widget relocation on position_control toggle
        let button = gtk::Button::with_label(">");
        button.set_hexpand(true);
        sg.add_widget(&button);

        let set_position = {
            let toggle = toggle.clone();
            let grid = grid.clone();
            let button = button.clone();

            move |toggle_on: bool| {
                grid.remove(&widget);
                grid.remove(&button);

                let on_left = toggle.on_position < toggle.off_position;
                button.set_label(if on_left ^ toggle_on { "<" } else { ">" });
                button.show();

                let left = if toggle_on { toggle.on_position } else { toggle.off_position } as i32;
                grid.attach(&button, left, 0, 1, 1);
                grid.attach(&widget, left, 1, 1, 1);
            }
        };
        set_position(false);

        // gui -> controller
        {
            let controller = controller.clone();
            let name = toggle.position_control.clone();
            button.connect_clicked(move |_| {
                let mut controller = controller.lock().unwrap();
                let v = controller.get(&name);
                if v.is_none() {
                    warn!("Toggle position control {:?} not found", &name);
                    return;
                }
                let v = v.unwrap() > 0;
                let v = !v; // toggling
                controller.set(&name, v as u16, StoreOrigin::UI);
            });
        }

        // controller -> gui
        {
            let controller = controller.clone();
            let name = toggle.position_control.clone();
            callbacks.insert(
                name.clone(),
                Rc::new(move || {
                    let v = {
                        let controller = controller.lock().unwrap();
                        controller.get(&name).unwrap()
                    };
                    set_position(v > 0);
                })
            )
        };
    }

    Ok(())
}