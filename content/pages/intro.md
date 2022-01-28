title: Introduction
save_as: index.html

![Screenshot]({static}/images/screenshot-small.png)

**pod-ui** is a modern cross-platform app to control Line6 POD 2.0
guitar modelling amp via MIDI. For pre-build binaries check out the 
[releases page](https://github.com/arteme/pod-ui/releases).

The app is a work-in-progress and many features are still missing,
however basic editing of the current preset already works. 
I intended to get feature parity with **Line6 Edit** on things like:

 - controlling the POD;
 - up-/downloading individual patches;
 - up-/downloading patch libraries;
 - loading `.l6t` and `.lib` files;

I would like to support other MIDI-based Line6 products, such as
PODxt, Bass POD, etc. I do not own any of these and would need
volunteers to test.

## Development

This is a GTK+-based app written in rust and distributed under GPLv3
license. The sources can be found from [github](https://github.com/arteme/pod-ui/).
Please feel free to open issues there in case of bugs or inconsistencies
with Line6 Edit.

The app is developed using MIDI controls references and SysEx protocol
descriptions published by Line6 and my own POD 2.0 device.

The app is developed in Linux, pre-build binaries are provided for
Windows (MacOS builds are in the plans).
