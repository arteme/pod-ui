use git_version::git_version;

const GIT_VERSION: &str = git_version!(args = ["--tags", "--always", "--match", "v*", "--dirty=+"],
                                      fallback = "");
fn set_git_version_env() {
    println!("cargo:rustc-env=GIT_VERSION={}", GIT_VERSION);
}

const SENTRY_DSN: &str = "https://bf6f7de248b147dea1fb48c221f785f0@o1148278.ingest.sentry.io/6219700";
fn set_sentry_dsn_env() {
    if matches!(option_env!("SENTRY"), Some(x) if x == "1") {
        println!("cargo:rustc-env=SENTRY_DSN={}", SENTRY_DSN);
    }
}


#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn platform_specific() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/icon.ico");
    res.compile().unwrap();

    set_git_version_env();
}

#[cfg(unix)]
fn platform_specific() {
}

fn main() {
    platform_specific();
    set_git_version_env();
    set_sentry_dsn_env();
}
