use std::borrow::Borrow;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use log::*;
use anyhow::*;
use glib::SignalHandlerId;

use gtk::prelude::*;
use pod_core::config::GUI;
use pod_core::controller::{Controller, ControllerStoreExt};
use pod_core::model::{Control, Format};
use pod_core::store::*;
use crate::{Callbacks, ObjectList};

pub struct SignalHandler {
    handler_id: SignalHandlerId,
    object: glib::Object
}

pub trait SignalHandlerExt {
    fn blocked<F: Fn() -> R,R>(&self, f: F) -> R;
}

impl SignalHandler {
    pub fn new<T: ObjectType>(instance: &T, handler_id: SignalHandlerId) -> Self {
        Self { handler_id, object: instance.clone().dynamic_cast::<glib::Object>().unwrap() }
    }

    pub fn block(&self) {
        glib::signal::signal_handler_block(&self.object, &self.handler_id);
    }

    pub fn unblock(&self) {
        glib::signal::signal_handler_unblock(&self.object, &self.handler_id);
    }

    pub fn blocked<T: Borrow<SignalHandler>, F: Fn() -> R,R>(handlers: &[T], f: F) -> R {
        for handler in handlers {
            handler.borrow().block();
        }
        let r = f();
        for handler in handlers {
            handler.borrow().unblock();
        }

        r
    }
}

impl SignalHandlerExt for SignalHandler {
    fn blocked<F: Fn() -> R, R>(&self, f: F) -> R {
        SignalHandler::blocked(&[self], f)
    }
}

impl SignalHandlerExt for Vec<SignalHandler> {
    fn blocked<F: Fn() -> R, R>(&self, f: F) -> R {
        let s = self.as_slice();
        SignalHandler::blocked(s, f)
    }
}

pub fn wire(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {

    objs.named_objects()
        .for_each(|(obj, name)| {
            {
                let controller = controller.lock().unwrap();
                if !controller.has(&name) {
                    warn!("Not wiring {:?}", name);
                    return;
                }
            }
            info!("Wiring {:?} {:?}", name, obj);
            obj.dynamic_cast_ref::<gtk::Scale>().map(|scale| {
                // wire GtkScale and its internal GtkAdjustment
                let adj = scale.adjustment();
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::RangeControl(c)) => {
                            let (from, to) = c.bounds();
                            adj.set_lower(from);
                            adj.set_upper(to);

                            match &c.format {
                                Format::Callback(f) => {
                                    let c = c.clone();
                                    let f = f.clone();
                                    scale.connect_format_value(move |_, val| f(&c, val));
                                },
                                Format::Data(data) => {
                                    let data = data.clone();
                                    scale.connect_format_value(move |_, val| data.format(val));
                                },
                                Format::Labels(labels) => {
                                    let labels = labels.clone();
                                    scale.connect_format_value(move |_, val| labels.get(val as usize).unwrap_or(&"".into()).clone());

                                }
                                _ => {}
                            }
                        },
                        _ => {
                            warn!("Control {:?} is not a range control!", name)
                        }
                    }
                }
                let handler;

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    let h = adj.connect_value_changed(move |adj| {
                        let mut controller = controller.lock().unwrap();
                        controller.set(&name, adj.value() as u16, GUI);
                    });
                    handler = SignalHandler::new(&adj, h);
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    callbacks.insert(
                        name.clone(),
                        Rc::new(move || {
                            // TODO: would be easier if value is passed in the message and
                            //       into this function without the need to look it up from the controller
                            let v = {
                                let controller = controller.lock().unwrap();
                                controller.get(&name).unwrap()
                            };
                            handler.blocked(|| adj.set_value(v as f64));
                        })
                    )
                }
            });
            obj.dynamic_cast_ref::<gtk::CheckButton>().map(|check| {
                // HACK: DO NOT PROCESS RADIO BUTTONS HERE!
                if obj.dynamic_cast_ref::<gtk::RadioButton>().is_some() { return }
                // wire GtkCheckBox
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::SwitchControl(_)) => {},
                        _ => {
                            warn!("Control {:?} is not a switch control!", name)
                        }
                    }
                }
                let handler;

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    let h = check.connect_toggled(move |check| {
                        controller.set(&name, check.is_active() as u16, GUI);
                    });
                    handler = SignalHandler::new(check, h);
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    let check = check.clone();
                    callbacks.insert(
                        name.clone(),
                        Rc::new(move || {
                            let v = controller.get(&name).unwrap();
                            handler.blocked(|| check.set_active(v > 0));
                        })
                    )
                }
            });
            obj.dynamic_cast_ref::<gtk::RadioButton>().map(|radio| {
                // wire GtkRadioButton
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::SwitchControl(_)) => {},
                        _ => {
                            warn!("Control {:?} is not a switch control!", name)
                        }
                    }
                }
                let mut handlers = Vec::<SignalHandler>::new();

                // this is a group, look up the children
                let group = radio.group();

                // wire gui -> controller
                for radio in group.clone() {
                    let controller = controller.clone();
                    let name = name.clone();
                    let radio_name = ObjectList::object_name(&radio).unwrap();
                    let value = radio_name.find(':')
                        .map(|pos| &radio_name[pos+1..]).map(|str| str.parse::<u16>().unwrap());
                    if value.is_none() {
                        // value not of "name:N" pattern, skip
                        continue;
                    }
                    let h = radio.connect_toggled(move |radio| {
                        if !radio.is_active() { return; }
                        let mut controller = controller.lock().unwrap();
                        controller.set(&name, value.unwrap(), GUI);
                    });
                    handlers.push(SignalHandler::new(&radio, h));
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    callbacks.insert(
                        name.clone(),
                        Rc::new(move || {
                            let v = {
                                let controller = controller.lock().unwrap();
                                controller.get(&name).unwrap()
                            };
                            let item_name = format!("{}:{}", name, v);
                            group.iter().find(|radio| ObjectList::object_name(*radio).unwrap_or_default() == item_name)
                                .and_then(|item| {
                                    handlers.blocked(|| item.set_active(true));
                                    Some(())
                                })
                                .or_else( || {
                                    error!("GtkRadioButton not found with name '{}'", name);
                                    None
                                });
                        })
                    )
                }
            });
            obj.dynamic_cast_ref::<gtk::ComboBoxText>().map(|combo| {
                // wire GtkComboBox
                let controller = controller.clone();
                match controller.get_config(&name) {
                    Some(Control::Select(c)) => Some(c),
                    _ => {
                        warn!("Control {:?} is not a select control!", name);
                        None
                    }
                };
                let handler;

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    let h = combo.connect_changed(move |combo| {
                        combo.active().map(|v| {
                            controller.set(&name, v as u16, GUI);
                        });
                    });
                    handler = SignalHandler::new(combo, h);
                }
                // wire controller -> gui
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    let combo = combo.clone();
                    callbacks.insert(
                        name.clone(),
                        Rc::new(move || {
                            let v = controller.get(&name).unwrap() as u16;
                            handler.blocked(|| combo.set_active(Some(v as u32)));
                        })
                    )
                }
            });
            obj.dynamic_cast_ref::<gtk::Button>().map(|button| {
                // wire GtkButton
                let controller = controller.clone();
                {
                    let controller = controller.lock().unwrap();
                    match controller.get_config(&name) {
                        Some(Control::Button(_)) => {},
                        _ => {
                            warn!("Control {:?} is not a button!", name);
                            return;
                        }
                    }
                }

                // wire gui -> controller
                {
                    let controller = controller.clone();
                    let name = name.clone();
                    button.connect_clicked(move |_button| {
                        let mut controller = controller.lock().unwrap();
                        controller.set_full(&name, 1, GUI, Signal::Force);
                    });
                }
                // wire controller -> gui
                // Nothing here. This is UI-only!
            });
        });

    Ok(())
}
