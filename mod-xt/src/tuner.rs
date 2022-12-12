use std::time::Duration;
use pod_core::context::Ctx;
use pod_core::event::{AppEvent, SenderExt};
use pod_core::midi::MidiMessage;

pub struct Tuner {
    handle: Option<tokio::task::JoinHandle<()>>
}

impl Tuner {
    pub fn new() -> Self {
        Self {
            handle: None
        }
    }

    pub fn start(&mut self, ctx: &Ctx) -> bool {
        if let Some(_) = self.handle {
            return false;
        }

        let app_event_tx = ctx.app_event_tx.clone();
        let handle = tokio::spawn(async move {
            loop {
                app_event_tx.send_or_warn(AppEvent::MidiMsgOut(MidiMessage::XtTunerNoteRequest));
                app_event_tx.send_or_warn(AppEvent::MidiMsgOut(MidiMessage::XtTunerOffsetRequest));
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        });
        self.handle.replace(handle);
        true
    }

    pub fn stop(&mut self) -> bool {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            return true;
        }
        false
    }
}

impl Drop for Tuner {
    fn drop(&mut self) {
        self.stop();
    }
}