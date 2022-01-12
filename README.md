# POD UI

**DISCLAIMER: This software is alpha quality. You have been warned!**

A GTK+ application for controlling Line6 [POD 2.0](https://www.musikhaus-korn.de/en/line6-pod-20/pd/15909)
guitar modelling amp via MIDI. The app is written in Rust and is as much a
project of learning Rust as actually doing what the app is supposed to do.

The UI is inspired by [qtpod](https://llg.cubic.org/tools/qtpod/). 

I intend to get feature parity with *Line6 Edit* on things like:
 * controlling the amp itself;
 * up-/downloading individual tones and full device patch set via
   *SysEx* messages;
 * loading `.l6t` and `.lib` files;

Potentially, I would like to support other POD models as well.

## Why?

Because I have a POD 2.0 device and I want to control it from my computer!
Moreover, I have a Linux system. This provides a whole lot of inconvenience:
 * the original *Line6 Edit* application is Windows/Mac-only and is hard to 
   find and run nowadays;
 * *Podman32* is Windows-only and hard to find and run much like the above;
 * *Qtpod* is a QT3 application. No-one has QT3 anymore.

What is a programmer to do? Write their own app, of course!


