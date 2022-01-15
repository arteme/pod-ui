#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/icon.ico");
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {
}