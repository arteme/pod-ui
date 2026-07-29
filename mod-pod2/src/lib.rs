mod config;
mod module;
pub mod wiring;
pub mod handler;

pub use module::*;

/// GTK only tolerates being touched from a single thread for the life of the
/// process - not just `init()`, every call. Rust's test harness runs each
/// `#[test]` fn on its own thread, so this crate can only have ONE test that
/// touches GTK; see `gtk_smoke_tests` below, which calls every check that
/// needs GTK from that single thread instead of each being its own test.
/// `Once` just makes this safe to call more than once from that one thread.
#[cfg(test)]
pub(crate) fn ensure_gtk_init() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        pod_gtk::prelude::gtk::init().expect("gtk::init failed - is a display available?");
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn gtk_smoke_tests() {
        crate::ensure_gtk_init();
        // registers the "pod2/" dev override as a side effect - must run
        // before check_pod_conf_covers_all_controls, which relies on it
        crate::module::tests::check_module_inits_via_resource_resolution();
        crate::config::tests::check_pod_conf_covers_all_controls();
    }
}
