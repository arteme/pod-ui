use std::sync::{Arc, Mutex};
use pod_gtk::ObjectList;
use crate::{State, UIEvent};

pub fn wire_panic_indicator(state: Arc<Mutex<State>>) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        prev(info);

        // send a panic event to the UI thread
        state.lock().unwrap().ui_event_tx.send(UIEvent::Panic);
    }));
}