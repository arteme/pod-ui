title: Introduction
url: 
save_as: index.html

![Screenshot]({static}/images/pod-ui-v1.0.0-podxt.jpg)

**pod-ui** is a modern cross-platform app to control Line6 POD family
of guitar modelling amps via MIDI. Currently, it supports **POD 2.0**,
**POD Pro**, **PocketPOD** and a family of **PODxt** devices (PODxt,
PODxt Pro, PODxt Live).
For pre-built binaries check out the 
[releases page](https://github.com/arteme/pod-ui/releases).

The app is a work-in-progress and many features may still be missing,
however I intended to get feature parity with **Line6 Edit** on things like:

 - ☑ controlling the POD;
 - ☑ up-/downloading individual patches;
 - ☑ up-/downloading patch libraries;
 - ☐ loading `.l6t` and `.lib` files;
 - ☑ support for POD device **★new in version 1.3.0★**;
 - ☑ support for POD 2.0/POD Pro device;
 - ☑ support for Pocket POD device;
 - ☑ support for PODxt/PODxt Pro/PODxt Live device;
 - ☑ support for Bass PODxt/Bass PODxt Pro/Bass PODxt Live device **★new in version 1.3.0★**;
 - ☐ support for Bass POD device;
 - ☐ support for Floor POD Plus device;
 - ☐ support for [other compatible]({filename}devices.md) Line6 devices;

I would like to support other MIDI-based legacy Line6 products, such as
Bass POD, Floor POD, etc. I do not own any of these and would need
volunteers to test. If you would like to volunteer for this, please open
an issue at the [issues page](https://github.com/arteme/pod-ui/issues). 

## Development

This is a GTK+-based app written in rust and distributed under GPLv3
license. The sources can be found from [github](https://github.com/arteme/pod-ui/).
Please feel free to open issues there in case of bugs or inconsistencies
with Line6 Edit.

The app is developed using MIDI controls references and SysEx protocol
descriptions published by Line6 and my own POD devices.

The app is developed in Linux, pre-build binaries are provided for
Windows and MacOS.
