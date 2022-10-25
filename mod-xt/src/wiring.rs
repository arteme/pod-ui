use std::sync::{Arc, Mutex};
use pod_core::store::{Signal, Store};
use pod_core::controller::Controller;
use pod_core::model::{AbstractControl, Config};
use pod_gtk::prelude::*;
use anyhow::*;
use log::*;
use pod_core::config::{GUI, MIDI};
use pod_gtk::logic::LogicBuilder;
use crate::{config, model};
use crate::config::XtPacks;

fn is_sensitive(packs: XtPacks, name: &str) -> bool {
    let ms = name.starts_with("MS-");
    let cc = name.starts_with("CC-");
    let bx = name.starts_with("BX-");
    let fx = name.starts_with("FX-");

    (!ms && !cc && !bx && !fx) ||
        (ms && packs.contains(XtPacks::MS)) ||
        (cc && packs.contains(XtPacks::CC)) ||
        (bx && packs.contains(XtPacks::BX)) ||
        (fx && packs.contains(XtPacks::FX))
}

fn init_combo(packs: XtPacks, objs: &ObjectList, name: &str, items: Vec<&str>) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBox>(name)?;

    let list_store = gtk::ListStore::new(
        &[u8::static_type(), String::static_type(), bool::static_type()]
    );

    for (i, item) in items.iter().enumerate() {
        let sensitive = is_sensitive(packs, item);
        list_store.insert_with_values(None, &[
            (0, &(i as u32)), (1, item), (2, &sensitive)
        ]);
    }

    select.set_model(Some(&list_store));
    select.clear();
    select.set_entry_text_column(1);

    let renderer = gtk::CellRendererText::new();
    select.pack_start(&renderer, true);
    select.add_attribute(&renderer, "text", 1);
    select.add_attribute(&renderer, "sensitive", 2);

    Ok(())
}

pub fn init_amp_models(packs: XtPacks, objs: &ObjectList, config: &Config) -> Result<()> {
    let items = config.amp_models.iter().map(|a| a.name.as_str()).collect::<Vec<_>>();
    return init_combo(packs, objs, "amp_select", items);
}
pub fn init_cab_models(packs: XtPacks, objs: &ObjectList, config: &Config) -> Result<()> {
    let items = config.cab_models.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    return init_combo(packs, objs, "cab_select", items);
}

// todo: when switching to BX cab update the mic names from BC_MIC_NAMES!
pub fn init_mic_models(objs: &ObjectList) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBox>("mic_select")?;

    let list_store = gtk::ListStore::new(
        &[u8::static_type(), String::static_type(), bool::static_type()]
    );

    for (i, item) in config::MIC_NAMES.iter().enumerate() {
        list_store.insert_with_values(None, &[
            (0, &(i as u32)), (1, item), (2, &true)
        ]);
    }

    select.set_model(Some(&list_store));
    select.clear();
    select.set_entry_text_column(1);

    let renderer = gtk::CellRendererText::new();
    select.pack_start(&renderer, true);
    select.add_attribute(&renderer, "text", 1);

    Ok(())
}

pub fn wire_stomp_select(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let param_names = vec![
        "stomp_param2", "stomp_param2_wave", "stomp_param2_octave",
        "stomp_param3", "stomp_param3_octave", "stomp_param3_offset",
        "stomp_param4", "stomp_param4_offset",
        "stomp_param5", "stomp_param6",
    ];

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        // wire `stomp_select` controller -> gui
        .on("stomp_select")
        .run(move |value, _, _| {
            let stomp_config = &(*config::STOMP_CONFIG)[value as usize];

            for param in param_names.iter() {
                let label_name = format!("{}_label", param);
                let label = objs.ref_by_name::<gtk::Label>(&label_name).unwrap();
                let widget = objs.ref_by_name::<gtk::Widget>(param).unwrap();

                if let Some(text) = stomp_config.labels.get(&param.to_string()) {
                    label.set_text(text);
                    label.show();
                    widget.show();
                } else {
                    label.hide();
                    widget.hide();
                }
            }
        })
        // any change on the `stomp_param2` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param2` as a value coming from GUI
        .on("stomp_param2")
        .run(move |value, controller, _| {
            let control = controller.get_config("stomp_param2_wave").unwrap();
            let value = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param2_wave", value, MIDI);

            let control = controller.get_config("stomp_param2_octave").unwrap();
            let value = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param2_octave", value, MIDI);
        })
        .on("stomp_param2_wave").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param2_wave").unwrap();
            let value = control.value_to_midi(value);
            controller.set("stomp_param2", value as u16, origin);
        })
        .on("stomp_param2_octave").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param2_octave").unwrap();
            let value = control.value_to_midi(value);
            controller.set("stomp_param2", value as u16, origin);
        })
        // any change on the `stomp_param3` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param3` as a value coming from GUI
        .on("stomp_param3")
        .run(move |value, controller, _| {
            let control = controller.get_config("stomp_param3_octave").unwrap();
            let value = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param3_octave", value, MIDI);

            let control = controller.get_config("stomp_param3_offset").unwrap();
            let value = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param3_offset", value, MIDI);
        })
        .on("stomp_param3_octave").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param3_octave").unwrap();
            let value = control.value_to_midi(value);
            controller.set("stomp_param3", value as u16, origin);
        })
        .on("stomp_param3_offset").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param3_offset").unwrap();
            let value = control.value_to_midi(value);
            controller.set("stomp_param3", value as u16, origin);
        })
        // any change on the `stomp_param4` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param4` as a value coming from GUI
        .on("stomp_param4")
        .run(move |value, controller, _| {
            let control = controller.get_config("stomp_param4_offset").unwrap();
            let value = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param4_offset", value, MIDI);
        })
        .on("stomp_param4_offset").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param4_offset").unwrap();
            let value = control.value_to_midi(value);
            controller.set("stomp_param4", value as u16, origin);
        });

    Ok(())

}