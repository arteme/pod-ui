use std::sync::{Arc, Mutex};
use pod_core::controller::*;
use pod_core::model::{AbstractControl, Config};
use pod_gtk::prelude::*;
use anyhow::*;
use log::*;
use pod_core::controller::StoreOrigin::UI;
use pod_gtk::logic::LogicBuilder;
use crate::config;
use crate::config::XtPacks;
use crate::widgets::*;


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

pub fn init_combo<T, F>(objs: &ObjectList, name: &str, list: &Vec<T>, get_name: F) -> Result<()>
    where F: Fn(&T) -> &str

{
    let select = objs.ref_by_name::<gtk::ComboBox>(name)?;

    let list_store = gtk::ListStore::new(
        &[u32::static_type(), String::static_type(), bool::static_type()]
    );

    for (i, item) in list.iter().enumerate() {
        let name = get_name(item);
        list_store.insert_with_values(None, &[
            (0, &(i as u32)), (1, &name), (2, &true)
        ]);
    }

    select.set_model(Some(&list_store));
    select.clear();

    let renderer = gtk::CellRendererText::new();
    select.pack_start(&renderer, true);
    select.add_attribute(&renderer, "text", 1);
    select.add_attribute(&renderer, "sensitive", 2);

    Ok(())
}

fn update_combo<F>(objs: &ObjectList, name: &str, update: F) -> Result<()>
    where F: Fn(u32, &str) -> (Option<String>, Option<bool>)
{
    let select = objs.ref_by_name::<gtk::ComboBox>(name)?;
    let model = select.model().unwrap();

    let list_store = model.dynamic_cast::<gtk::ListStore>().unwrap();
    list_store.foreach(|_, _, iter| {
        let idx = list_store.value(iter, 0);
        let idx = idx.get::<u32>().unwrap();

        let value = list_store.value(iter, 1);
        let value = value.get::<&str>().unwrap();

        let values = update(idx, value);

        if let Some(text) = values.0 {
            list_store.set_value(iter, 1, &text.to_value());
        }
        if let Some(sensitive) = values.1 {
            list_store.set_value(iter, 2, &sensitive.to_value());
        }

        false
    });

    Ok(())
}

pub fn wire_di_show(controller: Arc<Mutex<Controller>>, config: &'static Config, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        // wire `amp_select` for `di:show`
        .on("amp_select")
        .run(move |value, controller, origin| {
            let amp = config.amp_models.get(value as usize);
            if let Some(amp) = amp {
                let show = amp.name.starts_with("BX-") as u16;
                controller.set("di:show", show, origin);
            }
        });

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
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param2_wave").unwrap();
            let midi = control.value_from_midi(value as u8);
            controller.set("stomp_param2_wave", midi, origin);

            let control = controller.get_config("stomp_param2_octave").unwrap();
            let midi = control.value_from_midi(value as u8);
            controller.set("stomp_param2_octave", midi, origin);
        })
        .on("stomp_param2_wave").from(UI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param2_wave").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param2", midi as u16, origin);
        })
        .on("stomp_param2_octave").from(UI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param2_octave").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param2", midi as u16, origin);
        })
        // any change on the `stomp_param3` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param3` as a value coming from GUI
        .on("stomp_param3")
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param3_octave").unwrap();
            let midi = control.value_from_midi(value as u8);
            controller.set("stomp_param3_octave", midi, origin);

            let control = controller.get_config("stomp_param3_offset").unwrap();
            let midi = control.value_from_midi(value as u8);
            controller.set("stomp_param3_offset", midi, origin);
        })
        .on("stomp_param3_octave").from(UI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param3_octave").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param3", midi as u16, origin);
        })
        .on("stomp_param3_offset").from(UI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param3_offset").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param3", midi as u16, origin);
        })
        // any change on the `stomp_param4` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param4` as a value coming from GUI
        .on("stomp_param4")
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param4_offset").unwrap();
            let midi = control.value_from_midi(value as u8);
            controller.set("stomp_param4_offset", midi, origin);
        })
        .on("stomp_param4_offset").from(UI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param4_offset").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param4", midi as u16, origin);
        });

    Ok(())
}

pub fn wire_mod_select(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let param_names = vec!["mod_param2", "mod_param3", "mod_param4"];

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        // wire `mod_select` controller -> gui
        .on("mod_select")
        .run(move |value, _, _| {
            let mod_config = &(*config::MOD_CONFIG)[value as usize];

            for param in param_names.iter() {
                let label_name = format!("{}_label", param);
                let label = objs.ref_by_name::<gtk::Label>(&label_name).unwrap();
                let widget = objs.ref_by_name::<gtk::Widget>(param).unwrap();

                if let Some(text) = mod_config.labels.get(&param.to_string()) {
                    label.set_text(text);
                    label.show();
                    widget.show();
                } else {
                    label.hide();
                    widget.hide();
                }
            }
        });

    Ok(())
}

pub fn wire_delay_select(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let param_names = vec![
        "delay_param2",
        "delay_param3", "delay_param3_heads",
        "delay_param4", "delay_param4_bits",
    ];

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        // wire `delay_select` controller -> gui
        .on("delay_select")
        .run(move |value, _, _| {
            let config = &(*config::DELAY_CONFIG)[value as usize];

            for param in param_names.iter() {
                let label_name = format!("{}_label", param);
                let label = objs.ref_by_name::<gtk::Label>(&label_name).unwrap();
                let widget = objs.ref_by_name::<gtk::Widget>(param).unwrap();

                if let Some(text) = config.labels.get(&param.to_string()) {
                    label.set_text(text);
                    label.show();
                    widget.show();
                } else {
                    label.hide();
                    widget.hide();
                }
            }
        })
        // any change on the `delay_param3` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `delay_param3` as a value coming from GUI
        .on("delay_param3")
        .run(move |value, controller, origin| {
            let control = controller.get_config("delay_param3_heads").unwrap();
            let midi = control.value_from_midi(value as u8);
            controller.set("delay_param3_heads", midi, origin);
        })
        .on("delay_param3_heads").from(UI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("delay_param3_heads").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("delay_param3", midi as u16, origin);
        })
        // any change on the `delay_param4` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param4` as a value coming from GUI
        .on("delay_param4")
        .run(move |value, controller, origin| {
            let control = controller.get_config("delay_param4_bits").unwrap();
            let midi = control.value_from_midi(value as u8);
            controller.set("delay_param4_bits", midi, origin);
        })
        .on("delay_param4_bits").from(UI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("delay_param4_bits").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("delay_param4", midi as u16, origin);
        });

    Ok(())
}

pub fn wire_xt_packs(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let selects = vec![
        "amp_select", "cab_select", "stomp_select", "mod_select", "delay_select"
    ];

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        .on("xt_packs")
        .run(move |value, _, _| {
            let packs = XtPacks::from_bits(value as u8).unwrap();
            for name in selects.iter() {
                update_combo(&objs, name, |_, name| {
                    let sensitive = is_sensitive(packs, name);
                    (None, Some(sensitive))
                }).unwrap();
            }
        });

    Ok(())
}

pub fn wire_mics_update(controller: Arc<Mutex<Controller>>, config: &'static Config, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        .on("cab_select")
        .run(move |value, _, _| {
            let cab_name = config.cab_models.get(value as usize);
            if cab_name.is_none() {
                error!("Cab select invalid value: {}", value);
                return;
            }
            let is_bx = cab_name.unwrap().starts_with("BX-");
            let mics = if is_bx { &config::BX_MIC_NAMES } else { &config::MIC_NAMES };
            update_combo(&objs, "mic_select", |n, _| {
                let name = mics.get(n as usize).map(|v| v.as_str())
                    .unwrap_or(&"");
                (Some(name.into()), None)
            }).unwrap();
        });

    Ok(())
}

pub fn wire_pedal_assign(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    // Pedal assign is really a range control, but for the sake of showing it
    // as a select we do this Select <-> VirtualSelect mapping
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        .on("pedal_assign")
        .run(move |value, controller, origin| {
            let value: u16 = match value {
                0 ..= 41 => 0,
                42 ..= 85 => 1,
                _ => 2
            };
            controller.set("pedal_assign_select", value, origin);
        })
        .on("pedal_assign_select").from(UI)
        .run(move |value, controller, origin| {
            let value: u16 = match value {
                0 => 0,
                1 => 64,
                _ => 127
            };
            controller.set("pedal_assign", value, origin);
        });

    Ok(())
}

pub fn resolve_footswitch_mode_show(objs: &ObjectList, config: &Config) -> Result<()> {
    let show = config.member == config::PODXT_LIVE_CONFIG.member;
    if show { return Ok(()); }

    // For some reason, hiding these particular controls wia `widget.hide()` leaves
    // extra space in the gtk::Frame, which I can't get rid of. Instead, we remove
    // them from the UI altogether.
    objs.widgets_by_class_match(&|class_name| class_name.starts_with("footswitch_mode:show"))
        .for_each(|(widget, _)| {
            let container = widget.parent()
                .and_then(|w| w.dynamic_cast::<gtk::Container>().ok())
                .unwrap();
            container.remove(widget);
        });

    Ok(())
}

pub fn wire_tuner(tuner: Tuner,
                  controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        .data(tuner)
        .on("tuner_offset")
        .run(move |value, _, _, tuner| {
            if value == 97 {
                tuner.set_offset(None);
            } else {
                let value = (value as i16).min(50).max(-50) as f64 / 50.0;
                tuner.set_offset(Some(value as f64));
            }
        })
        .on("tuner_note")
        .run(move |value, _, _, tuner| {
            if value == 0xfffe {
                tuner.set_note(None);
            } else {
                tuner.set_note(Some(value as usize));
            }
        });

    Ok(())
}
