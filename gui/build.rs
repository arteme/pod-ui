
extern crate git_version;

const GIT_VERSION: &str = git_version::git_describe!("--tags", "--always", "--dirty=+");
fn set_git_version_env() {
    println!("cargo:rustc-env=GIT_VERSION={}", GIT_VERSION);
}

#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/icon.ico");
    res.compile().unwrap();

    set_git_version_env();
}

#[cfg(unix)]
fn main() {
    set_git_version_env();
}