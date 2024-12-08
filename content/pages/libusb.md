title: Plain USB without drivers

As of version 1.5.0, we've added support for communicating with USB-based
POD devices such as PocketPOD, PODxt and Bass PODxt over plain USB without
the need for Line6 device drivers. This is primarily aimed at macOS users
that have an Apple Silicon Mac, however it will also be useful to people
that do not want to install Line6 device drivers on an Intel Mac or on
Windows.

While this feature was developed in Linux and also works there (and probably
works best in Linux), all Linux distributions typically come with drivers for
Line6 hardware compiled as modules and this functionality is simply not needed
in Linux.

## General notes

`pod-ui` will try its best to work with devices over plain USB in the same way
as with standard MIDI devices. It will try to auto-detect MIDI devices and,
of not found, it will try to auto-detect USB devices.

The settings dialog "MIDI in" and "MIDI out" drop-down menus contain separate
"MIDI" and "USB" sections and if a compatible USB device is detected and can
be opened, it will be selectable in the "USB" section:
![USB device successfully opened]({static}/images/libusb/usb-success.png){: width=55%}

If a compatible USB device cannot be opened, there will be an error message
about that device in the "USB" section:
![USB device open failed]({static}/images/libusb/usb-failure.png)

The USB devices are checked when the settings dialog is opened, ff the USB 
device you connect is not found from the MIDI in/out selection drop-downs,
close the settings dialog and open it again.

**Unfortunately, working with plain USB in most OSes is very finicky. If your
device doesn't get auto-detected or doesn't work correctly, try again!**
Sometimes it helps to:

- Unplug and plug the device back in;
- Restart `pod-ui`;
- Try auto-detection anew;
- Try selecting the device manually;

If nothing works, please open an issue at the [project issues tracker](https://github.com/arteme/pod-ui/issues).

## macOS

If you do not have the Line6 device drivers installed on your macOS,
`pod-ui` should just work with your PODs over USB. If you have Line6
drivers, the POD will be detected, but you won't be able to connect
to it over USB. You'll still be able to use it using MIDI that the
Line6 device drivers provide.

## Windows

To use this "communication over raw USB" functionality in Windows you
still need to install a driver. The difference is that it is not a Line6
device driver, but a driver that the `libusb` can communicate with.
Please use [Zadiq](https://zadig.akeo.ie/), an automated USB driver
installation GUI to install a generic USB driver for your POD:

![install winusb driver using zadiq]({static}/images/libusb/windows-zadiq-podxt.png)

After that, `pod-ui` will be able to communicate with the POD using
USB. Note, that there is a list of USB drivers to select from: `WinUSB`,
`libusb-win32`, `libusbK`. In my test, `WinUSB` works fine, but if it
doesn't work for you, please try a different USB driver.

For more information on the specifics of the `libusb` Windows back-end,
please see the dedicated [libusb wiki pages](https://github.com/libusb/libusb/wiki/Windows).
