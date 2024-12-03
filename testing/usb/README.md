# pod-ui USB testing app

This is a simple app to test the `pod-ui` USB handling code. It emulates
a custom USB pod device that the `pod-ui` connects to and tries to
communicate with using libusb/rusb-based code as if it were a normal
USB-bases POD.

## Run

To run this test app you need Linux with USB Device Controller (UDC) and
Dummy HCD/UDC module. These are typically found in all major Linux distros.

Run on as follows:

```
cargo build

sudo modprobe dummy_hcd
sudo target/debug/usb
```

If the tester app doesn't crash, it will connect to the Linux USB sub-sysytem
and will be available as a typical USB device:

```
~/pod-ui $ lsusb
 
Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 002 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
Bus 003 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 003 Device 002: ID 0c45:6a22 Microdia Integrated_Webcam_HD
Bus 003 Device 003: ID 27c6:63ac Shenzhen Goodix Technology Co.,Ltd. Goodix USB2.0 MISC
Bus 003 Device 004: ID 8087:0033 Intel Corp. AX211 Bluetooth
Bus 004 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
Bus 005 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 005 Device 004: ID 0010:0001 POD-UI testing device
```

To connect yo it, as with any USB device, you'll need to either set up
proper device node ownership and permissions, or:

```
sudo chmod 777 /dev/bus/usb/*/*
```
