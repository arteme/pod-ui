title: USB-MIDI cables

**TL;DR** *Stay away from cheap USB-MIDI cables off the Internet!
Invest in a cable that is known to work correctly. See below for
some suggestions.* 

We're dealing with legacy hardware here &mdash; the original POD was released
in 1998, POD 2.0 came in 2000 and PODxt came in 2002. While PODxt already
features a USB port, the only way to connect older PODs to your computer
is using MIDI cables. MIDI cables require a sound card with MIDI IN/OUT
ports, which not many hobbyist guitar players have.

Luckily, one can nowadays get a relatively cheap USB-MIDI cable. The problem
is, not all of them will work.

MIDI standard is not very complex and contains a variety of short messages,
such as Control Change `CC` messages, which are sent when you press or
release a key on the synthesizer or turn a knob on a POD or Program Change
`PC` messages, which are sent when you switch presets on a POD. For anything
more complex, System Exclusive `SysEx` messages are used. These are long
manufacturer-specific messages that can are used to query for devices and
upload/download patches.

When buying cheap USB-MIDI cables off the Internet, please be aware that
there may be issues with the longer `SysEx` messages sending and receiving:

1. Data Loss: `SysEx` messages can be quite long and complex, and cheaper 
   USB-MIDI cables may not be able to reliably transmit all the data in
   the message. This can result in lost data or incomplete messages.

2. Timing Issues: `SysEx` messages require precise timing to be properly
   received by the target device, and cheap USB-MIDI cables may not be
   able to reliably maintain the required timing accuracy. This can lead
   to errors or failed messages.

3. Buffer Overflow: Some MIDI devices have limited buffer sizes, and
   sending a large `SysEx` message can cause the buffer to overflow. Cheap
   USB-MIDI cables may have buffer overflow issues, leading to failed or.
   missing messages.

In summary, sending `SysEx` messages using cheap Chinese USB MIDI cables can
be problematic, and it may be advisable to invest in a higher-quality cable
to ensure reliable and accurate data transfer.

When working with USB-MIDI cables, Windows users are generally advised to 
try the `winrt` build first as it uses a newer MIDI driver stack that is
in constant development and support and may work better in newer versions 
of Windows.

## What is known to work

From my own experience and the experience of the pod-ui users, here are
some USB-MIDI cables that are known to work:

**M-Audio MIDISport Uno** is a very affordable USB-MIDI cable, which
supports `SyxEx` message sending and works with all platforms.

**Bespeco BM100** is another cheap USB-MIDI cable that is known to work.
Window users **need** to use the `winrt` build to get it working.

## What may work

Here are some more options of good quality USB-MIDI cables that
the Interned agrees will support `SysEx` messages:

**Roland UM-ONE MK2** is not the cheapest cable around, but is
widely regarded as one of the most reliable and stable options for
when SysEx messages are concerned.

**iConnectivity mio** and **mio XC** are other examples of not so
cheap, but quality products that are reported to support `SysEx`
messages, MIDI Time Code `MTC` and other advanced MIDI features.

**Yamaha UX16** is one of the pricier options, but it is known
to work reliably with `SysEx` messages.

**MOTU FastLane** USB MIDI interface is not exactly a 1-to-1
USB-MIDI cable like the rest of the devices on the list, but for
the sake of completeness, it is worth to mention that is device
is known for  its low-latency performance and reliable `SysEx`
messaging capabilities.

## What doesn't work

![generic MIDI controller off AliExpress]({static}/images/usb-midi/chinese-usb-midi.png)
{.center}

The cheap generic cable you get off AliExpress that looks like this
doesn't work with `SysEx` messages. I know, because I have one.
Ironically, I bought it when I dusted off my POD 2.0 and started
working on pod-ui to not have to use an external USB sound card
with a proper MIDI cable. I've not been abl to get it to work
on any platforms, YMMV.

## Bluetooth MIDI adapters

Nowadays, there are plenty of wireless MIDI plugs that either
use Bluetooth or ad-hoc wireless communication with a dongle
you attach to a computer. Generally, these are more expensive
that the USB-MIDI cables.

I've not used any of these, but here is what the Internet has
to say. Please let me know if you've successfully used one
with a POD:

**M-Vave MS1** is a very cheap device that is reported to
work with `SysEx` messages.

**Yamaha MD-BT01** bluetooth MIDI adapter is supposed to support
`SysEx` messages.

**Roland WM-1** bluetooth MIDI adapter is supposed to work
well. According to the manufacturer's website, you also need
a `WM-1D` dongle for Windows.

**Quicco Sound mi.1** bluetooth MIDI adapter is supposed to
work.

## What can I do if my cable doesn't work with SysEx

If the cable doesn't support `SysEx` messages, you won't be able
to auto-detect the device using pod-ui (or test the connection
in the settings dialog). You won't be able to get the list of
patches from the device, and you won't be able to store/retrieve
individual patches from the device.

You will still be able to switch presets and adjust individual
controls. Pod-ui will also react when you switch presets on
the POD or adjust controls on it.