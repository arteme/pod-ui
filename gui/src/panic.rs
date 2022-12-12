use std::sync::{Arc, Mutex};
use crate::{State, UIEvent};
use crate::util::SenderExt;

pub fn wire_panic_indicator(state: Arc<Mutex<State>>) {
    let ui_event_tx = state.lock().unwrap().ui_event_tx.clone();
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        prev(info);

        // send a panic event to the UI thread
        ui_event_tx.send_or_warn(UIEvent::Panic);
    }));
}