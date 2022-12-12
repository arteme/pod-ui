use std::fmt::Debug;
use std::sync::atomic;
use log::warn;
use pod_gtk::prelude::glib;

pub trait ToSome {
    type Inner;
    fn some(self) -> Option<Self::Inner>;
}

impl <T> ToSome for T {
    type Inner = T;

    fn some(self) -> Option<Self::Inner> {
        Some(self)
    }
}

/// A virtual thread id to show in logs to be able to trace thread start/stop.
/// This is just a running number with no connection to the real thread id.

static THREAD_ID_COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
pub fn next_thread_id() -> usize {
    THREAD_ID_COUNTER.fetch_add(1, atomic::Ordering::SeqCst)
}

// Implement SenderExt for glib::Sender<T> by duplicating `pod_core::event::SenderExt<T>`
// for a rather bogus reason IMO of E0210 :( ...

pub trait SenderExt<T> {
    fn send_or_warn(&self, msg: T);
}

impl<T: Debug> SenderExt<T> for glib::Sender<T> {
    fn send_or_warn(&self, msg: T) {
        self.send(msg).unwrap_or_else(|err| {
            warn!("Message cannot be sent: {:?}", err.0);
        });
    }
}
