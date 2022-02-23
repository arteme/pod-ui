# POD UI

A GTK+ application for controlling Line6 [POD 2.0](https://www.musikhaus-korn.de/en/line6-pod-20/pd/15909)
guitar modelling amp via MIDI. The app is written in Rust and is as much a
project of learning Rust as actually doing what the app is supposed to do.

The UI is inspired by [qtpod](https://llg.cubic.org/tools/qtpod/). 

The aim is to get feature parity with *Line6 Edit* on things like:

 - [x] controlling the POD;
 - [x] up-/downloading individual patches;
 - [x] up-/downloading patch libraries;
 - [ ] loading .l6t and .lib files;
 - [x] support for POD 2.0 device;
 - [ ] support for PODxt/Bass POD/other Line6 devices;

### Why?

I have a POD 2.0 device and there are far more controls in it than there are
knobs on the device itself. Moreover, I have a Linux system. This provides a
whole lot of inconvenience:
 * the original *Line6 Edit* application is Windows/Mac-only and is hard to 
   find and run nowadays;
 * *Podman32* is Windows-only and hard to find and run much like the above;
 * *Qtpod* is a QT3 application. No-one has QT3 anymore.

What is a programmer to do? Write their own app, of course!

## Building and running

Building the code from source requires `git`, the `rust` toolchain 
([rustup](https://rustup.rs/) is a popular tool to get you started), the
Gtk+ 3.x libraries and goes as follows: 

```shell
git clone git@github.com/arteme/pod-ui.git
cd pod-ui
cargo build
cargo run
```

Windows and MacOS users may require additional toolchains installed, please
check the [Development doc](DEVELOPMENT.md).

