//! A small "virtual path" resolver for runtime resource files
//! (device layout config and glade files) so that they can be
//! shipped alongside the executable and overridden by end-users.
//!
//! `resolve("pod2/pod.conf")` will resolve a path to "pod2/pod.conf"
//! as "<exe dir>/pod2/pod.conf" if no overrides registered.
//!
//! `register("pod2/", "/random/path")` will register an override so
//! that the previous resolve would resolve to "/random/path/pod.conf".
//!
//! In development modules should map their resource dir to their
//! source dir.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use anyhow::{Context, Result};
use once_cell::sync::Lazy;

static OVERRIDES: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Registers an override for `key`. A `key` ending in `/` is a prefix
/// override - it redirects any lookup key that starts with it. Otherwise
/// it's an exact override for that one key. Registering the same `key`
/// again replaces the previous override.
pub fn register(key: &str, path: &str) {
    OVERRIDES.lock().unwrap().insert(key.to_string(), path.to_string());
}

/// Resolves `key` (e.g. `"xt/pod.glade"`) to a real filesystem path - see
/// module docs for the resolution rules.
pub fn resolve(key: &str) -> Result<PathBuf> {
    let rewritten = rewrite(key);
    let path = Path::new(&rewritten);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(exe_dir()?.join(path))
}

fn rewrite(key: &str) -> String {
    let overrides = OVERRIDES.lock().unwrap();

    if let Some(exact) = overrides.get(key) {
        return exact.clone();
    }

    overrides.iter()
        .filter(|(k, _)| k.ends_with('/') && key.starts_with(k.as_str()))
        .max_by_key(|(k, _)| k.len())
        .map(|(k, v)| format!("{}/{}", v.trim_end_matches('/'), &key[k.len()..]))
        .unwrap_or_else(|| key.to_string())
}

fn exe_dir() -> Result<PathBuf> {
    let exe = env::current_exe()
        .context("could not determine the running executable's path")?;
    exe.parent()
        .map(Path::to_path_buf)
        .context("running executable has no parent directory")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_exe_relative_path() {
        let resolved = resolve("resources_test_default/some.conf").expect("should resolve");
        assert_eq!(resolved, exe_dir().unwrap().join("resources_test_default/some.conf"));
    }

    #[test]
    fn exact_override_replaces_whole_key() {
        register("resources_test_exact/pod.glade", "/abs/blah.glade");
        let resolved = resolve("resources_test_exact/pod.glade").expect("should resolve");
        assert_eq!(resolved, PathBuf::from("/abs/blah.glade"));
    }

    #[test]
    fn prefix_override_replaces_matched_prefix_only() {
        register("resources_test_prefix/", "/abs/path1");
        let resolved = resolve("resources_test_prefix/pod.glade").expect("should resolve");
        assert_eq!(resolved, PathBuf::from("/abs/path1/pod.glade"));
    }

    #[test]
    fn relative_override_result_gets_exe_dir_prepended() {
        register("resources_test_relative/pod.glade", "relative/blah.glade");
        let resolved = resolve("resources_test_relative/pod.glade").expect("should resolve");
        assert_eq!(resolved, exe_dir().unwrap().join("relative/blah.glade"));
    }
}
