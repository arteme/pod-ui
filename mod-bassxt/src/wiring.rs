use std::sync::{Arc, Mutex};
use pod_core::controller::*;
use pod_core::controller::StoreOrigin::NONE;
use pod_gtk::{Callbacks, ObjectList};
use pod_gtk::logic::LogicBuilder;

pub fn wire_delay_controls_show(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> anyhow::Result<()> {

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        .on("delay_select")
        .run(move |value, controller, _| {
            let show = value <= 6;
            controller.set("delay_controls:show", show as u16, NONE);
        });

    Ok(())
}
