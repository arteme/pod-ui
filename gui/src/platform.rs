//! A collection of custom platform-specific hacks that do not make sense,
//! but empirically have been shown to be needed.
//!

#[cfg(target_os = "macos")]
pub fn raise_app_window() {
    osx::raise();
}

#[cfg(not(target_os = "macos"))]
pub fn raise_app_window() {
}

#[allow(non_snake_case)]
#[cfg(target_os = "macos")]
mod osx {
    use objc2::runtime::{Object, Class};
    use objc2::{class, msg_send, sel, sel_impl};

    /**
     * On macOS, gtk_window_present doesn't always bring the window to the foreground.
     * Based on this SO article, however, we can remedy the issue with some ObjectiveC
     * magic:
     * https://stackoverflow.com/questions/47497878/gtk-window-present-does-not-move-window-to-foreground
     *
     * Use objc2 crate to do just that.
     */
    pub fn raise() {
        unsafe {
            let NSApplication = class!(NSApplication);
            let app: *mut Object = msg_send![NSApplication, sharedApplication];

            let _: () = msg_send![app, activateIgnoringOtherApps:true];
        }
    }
}
