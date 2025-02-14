//! A collection of custom platform-specific hacks that do not make sense,
//! but empirically have been shown to be needed.
//!
//!
use anyhow::*;
use std::sync::OnceLock;
use bitflags::{bitflags, Flags};

pub use imp::*;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PlatformHackFlags: u8 {
        const OSX_RAISE           = 0x01;
    }
}

// platform-specific default hack flags

#[cfg(target_os = "linux")]
const DEFAULT_HACK_FLAGS: PlatformHackFlags = PlatformHackFlags::empty();

#[cfg(target_os = "macos")]
const DEFAULT_HACK_FLAGS: PlatformHackFlags =
    PlatformHackFlags::OSX_RAISE;

#[cfg(target_os = "windows")]
const DEFAULT_HACK_FLAGS: PlatformHackFlags = PlatformHackFlags::empty();

pub(in crate::platform) fn platform_hack_flags() -> &'static mut PlatformHackFlags {
    static mut FLAGS: OnceLock<PlatformHackFlags> = OnceLock::new();
    // Safety: this is NOT sound, but actually using this reference as
    // mutable is only even used from the CLI-options parsing code,
    // well before it is ever read.
    unsafe {
        FLAGS.get_or_init(|| DEFAULT_HACK_FLAGS);
        FLAGS.get_mut().unwrap()
    }
}

pub fn set_platform_hack_flags(flags_string: &str) -> Result<PlatformHackFlags> {
    let flags = platform_hack_flags();

    for str in flags_string.split(",") {
        let (add, str) = match &str[0..3] {
            "no-" => (false, &str[3..]),
            _ => (true, str)
        };
        let name = str.replace("-", "_").to_ascii_uppercase();
        let flag = PlatformHackFlags::from_name(&name)
            .ok_or_else(|| anyhow!("Platform hack {str:?}/{name} not found"))?;
        flags.set(flag, add);
    }

    Ok(flags.clone())
}

pub fn get_platform_hack_flags() -> String {
    let flags = platform_hack_flags();
    PlatformHackFlags::FLAGS.iter().flat_map(|flag| {
        if !flags.contains(flag.value().clone()) {
            return None;
        }
        Some(flag.name().to_ascii_lowercase().replace("_", "-"))
    })
        .collect::<Vec<_>>()
        .join(",")
}

mod imp {
    use pod_gtk::prelude::*;
    use crate::platform::platform_hack_flags;
    use crate::PlatformHackFlags;

    // PlatformHackFlags::OSX_RAISE

    #[cfg(target_os = "macos")]
    pub fn raise_app_window() {
        if platform_hack_flags().contains(PlatformHackFlags::OSX_RAISE) {
            osx::raise();
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn raise_app_window() {
        // nop
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
}
