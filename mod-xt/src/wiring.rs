use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use pod_core::controller::*;
use pod_core::model::{AbstractControl, Config};
use pod_gtk::prelude::*;
use anyhow::*;
use log::*;
use multimap::MultiMap;
use regex::Regex;
use tokio::time::Instant;
use pod_core::controller::StoreOrigin::*;
use pod_gtk::logic::LogicBuilder;
use pod_mod_pod2::wiring::wire_14bit;
use crate::config;
use crate::config::{NOTE_DURATION, XtPacks};
use crate::model::{ConfigAccess, DelayConfig, ModConfig, StompConfig};
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

pub fn wire_dynamic_select<T: ConfigAccess>(select_name: &str, configs: &'static [T],
                                            controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {

    let param_names = configs.iter()
        .flat_map(|c| c.labels().keys())
        .collect::<HashSet<_>>();

    debug!("wiring dynamic select: {:?} -> {:?}", select_name, param_names);
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        // wire `XXX_select` controller -> gui
        .on(select_name)
        .run(move |value, _, _| {
            let config = &configs[value as usize];

            for param in param_names.iter() {
                let label_name = format!("{}_label", param);
                let label = objs.ref_by_name::<gtk::Label>(&label_name).unwrap();
                let widget = objs.ref_by_name::<gtk::Widget>(param).unwrap();

                if let Some(text) = config.labels().get(&param.to_string()) {
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

pub fn wire_dynamic_params<T: ConfigAccess>(configs: &'static [T],
                                            controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    use pod_core::store::Origin;

    // NOTE: param_to_variant()/variant_to_param() assume the param control is 1:1 with
    // midi values and does not do value_from_midi()/value_to_midi() on the data read
    // written to the controller for that control (only for control variants)

    fn param_to_variant(variant: &String, value: u16, controller: &mut Controller, origin: Origin) {
        let control = controller.get_config(variant).unwrap();
        let midi = control.value_from_midi(value as u8);
        // When a control param value comes from MIDI, it should be forwarded to
        // the variants as MIDI. If it comes from UI (parameter main control or
        // via a "variant to parameter" wiring, the value should be forwarded to
        // the variants as "no action should be taken for it".
        let origin = match origin {
            MIDI => MIDI,
            _ => NONE
        };
        controller.set(variant, midi, origin);
    }

    fn variant_to_param(variant: &String, param: &String, value: u16, controller: &mut Controller, origin: Origin) {
        let control = controller.get_config(variant).unwrap();
        let midi = control.value_to_midi(value);
        controller.set(param, midi as u16, origin);
    }

    let param_names = configs.iter()
        .flat_map(|c| c.labels().keys())
        .collect::<HashSet<_>>();

    let mut param_mapping = MultiMap::<String, String>::new();
    let param_regex = Regex::new(r"(.*_param\d)_.*").unwrap();
    for name in param_names.iter() {
        if let Some(caps) = param_regex.captures(name) {
            param_mapping.insert(
                caps.get(1).unwrap().as_str().into(),
                caps.get(0).unwrap().as_str().into()
            )
        }
    }

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let mut builder = builder.on("xyz"); // convert a LogicBuilder to a LogicOnBuilder
    for (param, variants) in param_mapping.iter_all() {
        debug!("wiring dynamic controls: {:?} <-> {:?}", param, variants);
        builder
            // any change on the `XXX_paramX` will show up on the virtual
            // controls as a value coming from MIDI, GUI changes from virtual
            // controls will show up on `XXX_paramX` as a value coming from GUI
            .on(&param)
            .run({
                let variants = variants.clone();
                move |value, controller, origin| {
                    for variant in variants.iter() {
                        param_to_variant(variant, value, controller, origin)
                    }
                }
            });

        for variant in variants.iter() {
            builder
                .on(variant).from(UI)
                .run({
                    let variant = variant.clone();
                    let param = param.clone();
                    move |value, controller, origin| {
                        variant_to_param(&variant, &param, value, controller, origin)
                    }
                });
        }
    }

    Ok(())
}

pub fn wire_stomp_select(stomp_config: &'static [StompConfig],
                         controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    wire_dynamic_select("stomp_select", stomp_config,
                        controller.clone(), objs, callbacks)?;
    wire_dynamic_params(stomp_config, controller, objs, callbacks)?;

    Ok(())
}

pub fn wire_mod_select(mod_config: &'static [ModConfig],
                       controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    wire_dynamic_select("mod_select", mod_config,
                        controller.clone(), objs, callbacks)?;
    wire_dynamic_params(mod_config, controller, objs, callbacks)?;

    Ok(())
}

pub fn wire_delay_select(delay_config: &'static [DelayConfig],
                         controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    wire_dynamic_select("delay_select", delay_config,
                        controller.clone(), objs, callbacks)?;
    wire_dynamic_params(delay_config, controller, objs, callbacks)?;

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

pub fn resolve_footswitch_mode_show(objs: &ObjectList, show: bool) -> Result<()> {
    if show { return Ok(()); }

    // For some reason, hiding these particular controls wia `widget.hide()` leaves
    // extra space in the gtk::Frame, which I can't get rid of. Instead, we remove
    // them from the UI altogether.
    objs.widgets_by_class_match(|class_name| class_name.starts_with("footswitch_mode:show"))
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

pub fn wire_tempo(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {

    fn mod_note_to_mod_speed(mod_note: u16, tempo: u16, controller: &mut Controller) {
        if mod_note == 0 { return }
        if tempo < 300 || tempo > 2400 { return }

        // convert tempo & mod_note to Hz
        let v = &NOTE_DURATION.get(mod_note as usize).unwrap_or(&0.0);
        let hz = (tempo as f32/2400.0 * *v).max(0.1).min(15.0);

        // convert Hz to mod_speed value
        let (k, b) = (14.9/16383.0, 0.1);
        let mod_speed = (hz - b) / k;

        controller.set("mod_speed", mod_speed as u16, NONE);
    }

    fn reset_mod_note(controller: &mut Controller) {
        controller.set("mod_note_select", 0u16, MIDI);
    }

    fn delay_note_to_delay_time(delay_note: u16, tempo: u16, controller: &mut Controller) {
        if delay_note == 0 { return }
        if tempo < 300 || tempo > 2400 { return }

        // convert tempo & delay_note to ms
        let v = &NOTE_DURATION.get(delay_note as usize).unwrap_or(&0.0);
        let ms = (1000.0 * 2400.0/(tempo as f32 * *v)).max(20.0).min(2000.0);

        // convert ms to delay_time value
        let (k, b) = (1980.0/16383.0, 20.0);
        let mod_speed = (ms - b) / k;

        controller.set("delay_time", mod_speed as u16, NONE);
    }

    fn reset_delay_note(controller: &mut Controller) {
        controller.set("delay_note_select", 0u16, MIDI);
    }

    wire_14bit(controller.clone(), objs, callbacks,
               "tempo", "tempo:msb", "tempo:lsb",
               true)?;

    wire_tempo_tap(controller.clone(), objs)?;

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        .on("tempo")
        .run(move |value, controller, _| {
            let mod_note = controller.get("mod_note_select").unwrap();
            mod_note_to_mod_speed(mod_note, value, controller);

            let delay_note = controller.get("delay_note_select").unwrap();
            delay_note_to_delay_time(delay_note, value, controller);
        })
        .on("mod_note_select")
        .run(move |value, controller, _| {
            let tempo = controller.get("tempo").unwrap();
            mod_note_to_mod_speed(value, tempo, controller);
        })
        .on("mod_speed").from(MIDI).from(UI)
        .run(move |_, controller, _| {
            reset_mod_note(controller);
        })
        .on("delay_note_select")
        .run(move |value, controller, _| {
            let tempo = controller.get("tempo").unwrap();
            delay_note_to_delay_time(value, tempo, controller);
        })
        .on("delay_time").from(MIDI).from(UI)
        .run(move |_, controller, _| {
            reset_delay_note(controller);
        });

    Ok(())
}

fn wire_tempo_tap(controller: Arc<Mutex<Controller>>, objs: &ObjectList) -> Result<()> {

    let button = objs.ref_by_name::<gtk::Button>("tempo_tap_button")?;
    let last_click = Rc::new(RefCell::new(Instant::now()));
    button.connect_clicked(move |_| {
        let mut last_click = last_click.borrow_mut();
        let now = Instant::now();

        let ms = now.duration_since(*last_click).as_millis();
        let bpm = 60000.0/(ms as f32);

        *last_click = now;

        // button presses less frequent than 3sec apart we just ignore
        if bpm < 20.0  { return }
        let tempo = bpm.max(30.0).min(240.0) * 10.0;
        controller.set("tempo", tempo as u16, UI);
    });

    Ok(())
}