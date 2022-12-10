use std::sync::atomic;

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