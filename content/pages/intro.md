title: Introduction
save_as: index.html

![Screenshot]({static}/images/screenshot-small.png)

**pod-ui** is a modern cross-platform app to control Line6 POD family
of guitar modelling amps via MIDI. Currently it support **POD 2.0**,
**POD Pro** and **PocketPOD**, but support for more devices is in the works.
For pre-build binaries check out the 
[releases page](https://github.com/arteme/pod-ui/releases).

The app is a work-in-progress and many features may still missing,
however I intended to get feature parity with **Line6 Edit** on things like:

 - [x] controlling the POD;
 - [x] up-/downloading individual patches;
 - [x] up-/downloading patch libraries;
 - [ ] loading `.l6t` and `.lib` files;
 - [x] support for POD 2.0/POD Pro device;
 - [x] support for Pocket POD device **new in version 0.7.0!**;
 - [ ] support for PODxt/Bass POD/other Line6 devices;

I would like to support other MIDI-based Line6 products, such as
PODxt, Bass POD, etc. I do not own any of these and would need
volunteers to test.

## Development

This is a GTK+-based app written in rust and distributed under GPLv3
license. The sources can be found from [github](https://github.com/arteme/pod-ui/).
Please feel free to open issues there in case of bugs or inconsistencies
with Line6 Edit.

The app is developed using MIDI controls references and SysEx protocol
descriptions published by Line6 and my own POD 2.0 & Pocket POD device.

The app is developed in Linux, pre-build binaries are provided for
Windows and MacOS.
