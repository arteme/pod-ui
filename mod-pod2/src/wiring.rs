use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use log::*;
use anyhow::*;
use pod_core::controller::*;
use pod_core::controller::StoreOrigin::{MIDI, NONE, UI};
use pod_core::edit::EditBuffer;
use pod_core::is_valid_char;
use pod_core::model::*;
use pod_core::store::Origin;
use pod_gtk::logic::LogicBuilder;
use pod_gtk::prelude::*;

pub fn wire_14bit(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks,
                  control_name: &str, msb_name: &str, lsb_name: &str, big_endian: bool) -> Result<()> {
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        .on(control_name)
        .run({
            let lsb_name = lsb_name.to_string();
            let msb_name = msb_name.to_string();

            move |value, controller, origin| {
                let msb = (value & 0x3f80) >> 7;
                let lsb = value & 0x7f;

                // Make sure GUI event always generates both MSB and LSB MIDI messages
                let signal = if origin == UI { Signal::Force } else { Signal::Change };
                if big_endian {
                    // PODxt/L6E sends msb,lsb
                    controller.set_full(&msb_name, msb, origin, signal.clone());
                    controller.set_full(&lsb_name, lsb, origin, signal);
                } else {
                    // L6E sends lsb,msb; POD2.0 only sends msb
                    controller.set_full(&lsb_name, lsb, origin, signal.clone());
                    controller.set_full(&msb_name, msb, origin, signal);
                }
            }
        })
        .on(msb_name).from(MIDI).from(NONE)
        .run({
            let control_name = control_name.to_string();

            move |value, controller, origin| {
                let control_value = controller.get(&control_name).unwrap();
                let lsb = control_value & 0x7f;
                let control_value = ((value & 0x7f) << 7) | lsb;
                controller.set(&control_name, control_value, origin);
            }
        })
        .on(lsb_name).from(MIDI).from(NONE)
        .run({
            let control_name = control_name.to_string();

            move |value, controller, origin| {
                let control_value = controller.get(&control_name).unwrap();
                let msb = control_value & 0x3f80;
                let control_value = msb | (value & 0x7f);
                controller.set(&control_name, control_value, origin);
            }
        });

    Ok(())
}

pub fn wire_amp_select(controller: Arc<Mutex<Controller>>, config: &Config, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    // controller -> gui
    {
        let objs = objs.clone();
        let controller = controller.clone();
        let name = "amp_select".to_string();
        let amp_models = config.amp_models.clone();
        callbacks.insert(
            name.clone(),
            Rc::new(move || {
                let (presence, bright_switch) = {
                    let controller = controller.lock().unwrap();
                    let v = controller.get(&name).unwrap();
                    if let Some(amp) = amp_models.get(v as usize) {
                        (amp.presence, amp.bright_switch)
                    } else {
                        return;
                    }
                };

                // to have these animate calls after the callback animate call we
                // schedule a one-off idle loop function
                let objs = objs.clone();
                glib::idle_add_local(move || {
                    animate(&objs, "presence:show", presence as u16);
                    animate(&objs, "bright_switch:show", bright_switch as u16);
                    Continue(false)
                });
            })
        )
    };
    Ok(())
}

pub fn wire_name_change(edit: Arc<Mutex<EditBuffer>>, config: &Config, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let entry = objs.ref_by_name::<gtk::Entry>("program_name").unwrap();
    entry.set_max_length(config.program_name_length as i32);

    let handler;

    // override insert-text handler to filter out unsupported characters
    {
        let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::new(RefCell::new(None));
        let h = entry.connect_insert_text({
            let handler = handler.clone();
            move |entry, text, pos| {
                entry.stop_signal_emission_by_name("insert-text");

                // filter out characters [0,32), 96, (122,) from the text
                // L6E replaces 0 -> 32, 96 -> 95, ..31, 123.. -> 95, we'll just drop them
                let text = text.chars().filter(|c| is_valid_char(*c)).collect::<String>();

                if !text.is_empty() {
                    let borrow = (*handler).borrow();
                    let Some(handler) = borrow.as_ref() else { return };
                    entry.block_signal(handler);
                    entry.insert_text(&text, pos);
                    entry.unblock_signal(handler);
                }
            }
        });
        handler.borrow_mut().replace(h);
    }

    // gui -> controller
    {
        let edit = edit.clone();
        let h = entry.connect_changed(move |entry| {
            let str = entry.text();
            let mut edit = edit.lock().unwrap();
            edit.set_name(str.as_str());
        });
        handler = SignalHandler::new(&entry, h);
    }

    // controller -> gui
    {
        let name = "name_change".to_string();
        callbacks.insert(
            name.clone(),
            Rc::new(move || {
                let name = edit.lock().unwrap().name();
                let cur = entry.text().to_string();
                if cur.trim() != name {
                    handler.blocked(|| entry.set_text(&name));
                }
            })
        )
    };
    Ok(())
}

fn effect_entry_for_value<'a>(config: &'a Config, value: u16) -> Option<(&'a EffectEntry, bool, usize)> {
    config.effects.iter()
        .enumerate()
        .flat_map(|(idx, effect)| {
            let delay = effect.delay.as_ref()
                .filter(|e| (value == e.id as u16))
                .map(|e| (e, true, idx));
            let clean = effect.clean.as_ref()
                .filter(|e| (value == e.id as u16))
                .map(|e| (e, false, idx));
            delay.or(clean)
        })
        .next()
        .or_else(|| {
            warn!("Effect select mapping for value {} not found!", value);
            None
        })

}

fn effect_select_from_midi(config: &Config, controller: &mut Controller, origin: Origin) -> Option<EffectEntry> {

    let value = controller.get("effect_select:raw").unwrap();
    let entry = effect_entry_for_value(config, value);
    let Some((entry, delay, index)) = entry else {
        return None;
    };

    controller.set("delay_enable", delay as u16, origin);
    controller.set("effect_select", index as u16, origin);

    Some(entry.clone())
}

fn effect_select_from_gui(config: &Config, controller: &mut Controller) -> Option<EffectEntry> {

    let value = controller.get("effect_select").unwrap();
    let effect = &config.effects[value as usize];
    let delay_enable = controller.get("delay_enable").unwrap() != 0;

    let (delay, clean) = (effect.delay.as_ref(), effect.clean.as_ref());

    // if delay_enabled is set, try to set an effect with delay (fallback to clean),
    // otherwise try to set clean effect (fallback to effect with delay)
    let entry =
        (if delay_enable { delay.or(clean) } else { clean.or(delay) })
            .unwrap().clone();

    controller.set("effect_select:raw", entry.id as u16, UI);

    Some(entry)
}
/*
fn effect_select_send_controls(controller: &mut Controller, effect: &EffectEntry) {
    for name in &effect.controls {
        if let Some(v) = controller.get(name) {
            controller.set_full(name, v, UI, Signal::Force);
        }
    }
}
*/
pub fn wire_effect_select(config: &Config, controller: Arc<Mutex<Controller>>, callbacks: &mut Callbacks) -> Result<()> {
    let objs = ObjectList::new(&gtk::Builder::new());
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);

    builder
        .data(config.clone())
        // effect_select: raw -> controller
        .on("effect_select:raw")
        .run(move |_, controller, origin, config| {
            effect_select_from_midi(config, controller, origin);
        })
        // effect_select: controller -> raw
        .on("effect_select")
        .run(move |_, controller, _, config| {
            if let Some(_e) = effect_select_from_gui(&config, controller) {
                /*
                // POD sends controls after effect select
                // Line6 Edit requests an edit buffer dump from the device
                effect_select_send_controls(&mut controller, &e);
                */
            }
        })
        // delay_enable: controller -> raw
        .on("delay_enable").from(UI)
        .run(move |value, controller, origin, config| {
            if value != 0 {
                let effect_select = controller.get("effect_select:raw").unwrap();
                let (_, delay, idx) =
                    effect_entry_for_value(&config, effect_select).unwrap();
                if !delay {
                    // if `delay_enable` was switched on in the UI and if coming from
                    // an effect which didn't have delay to begin with, check if it can
                    // have a delay at all (POD 2.0 rotary cannot). If not, then switch
                    // to plain "delay" effect.
                    let need_reset = config.effects.get(idx)
                        .map(|e| e.delay.is_none()).unwrap_or(false);
                    if need_reset {
                        controller.set("effect_select", 0u16, origin);
                    }
                }
            }
        })
        // effect_tweak
        .on("effect_tweak").from(MIDI).from(NONE)
        .run(move |value, controller, origin, config| {
            let effect_select = controller.get("effect_select:raw").unwrap();
            let (entry, _, _) =
                effect_entry_for_value(&config, effect_select).unwrap();
            let control_name = &entry.effect_tweak;
            if control_name.is_empty() {
                return;
            }

            // HACK: as if everything's coming straight from MIDI
            let config = controller.get_config("effect_tweak").unwrap();
            let val = config.value_to_midi(value);

            let config = controller.get_config(&control_name).unwrap();
            let val = config.value_from_midi(val);
            controller.set(&control_name, val, origin);
        });

    Ok(())
}
