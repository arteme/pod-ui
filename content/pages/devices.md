title: Devices

Here is a list of legacy Line6 devices that are known to be supported by pod-ui,
possible to be supported, or known to not be supported.

If you have a device that can be supported by pod-ui, but is not currently
supported, please consider helping out the project by 
[becoming a tester]({filename}help.md).

Device                            | Supported | Can be supported 
----------------------------------|-----------|------------------
[POD](#pod1)                      |           | ✓
[POD 2.0](#pod2)                  | ✓         |
[POD Pro](#pod2)                  | ✓         |
[Bass POD](#bass-pod)             |           | ✓
[PODxt](#podxt)                   | ✓         |
[PODxt Pro](#podxt)               | ✓         |
[PODxt Live](#podxt)              | ✓         |
[Bass PODxt](#bass-podxt)         |           | ✓
[Bass PODxt Pro](#bass-podxt)     |           | ✓
[Bass PODxt Live](#bass-podxt)    |           | ✓
[POD X3](#pod-x3)                 |           | ✗
[POD X3 Pro](#pod-x3)             |           | ✓
[POD X3 Live](#pod-x3)            |           | ✓
[Pocket POD](#pocket-pod)         | ✓         |
[Pocket POD Express](#pocket-pod) |           | ✗
[Floor POD](#floor-pod)           |           | ✗
[Floor POD Plus](#floor-pod)      |           | ✓
[Bass Floor POD](#bass-floor-pod) |           | ✗
[POD HD family](#pod-hd-family)   |           | ???
[HD147](#hd147)                   |           | ✓
[Vetta II family](#other)         |           | ✓
[Flextone III family](#other)     |           | ✓
[DT-series devices](#dt-series)   |           | ✓

<!-- ✓ ✗ heavy ✔ ✘ -->

### <a name=pod1></a> POD 1.0

**POD 1.0**, a.k.a the original Line6 POD. 28 amp models, only 16 accessible
from the device controls, the rest accessible via MIDI. The POD 1.0 device can
easily be supported, but they are so very rare nowadays that a device (or its
owner) can hardly be found in the wild.

### <a name=pod2></a> POD 2.0, POD Pro

**POD 2.0** and **POD Pro** are fully supported.

### Bass POD

**Bass POD** device can easily be supported if there is demand or testers
with this device. The device is similar to POD 2.0 and the SysEx message
layout is well-known.

### PODxt

**PODxt**, **PODxt Pro**, **PODxt Live** are supported since version 1.0.0
with minor functionality missing:

- clip indicator is not currently shown;
- PODxt Live Variax device controls not supported;
- Amp, FX presets cannot be edited in pod-ui;

### Bass PODxt

**Bass PODxt**, **Bass PODxt Pro**, **Bass PODxt Live** devices can be
supported as they are similar to the **PODxt** family of devices. Testers
needed.

### POD X3

**POD X3** device does not support MIDI controls and cannot be supported.
Other devices in the POD X3 family -- **POD X3 Live**, **POD X3 Pro** --
support MIDI controls and can, therefore, be supported. Testers needed.

### Pocket POD

**Pocket POD** device is fully supported. **Pocket POD Express** device
does not connect to a computer and as such cannot be controlled via
MIDI, so it is not supported.

### Floor POD

**Floor POD** device does not have MIDI ports and cannot be supported.
**Floor POD Plus** device has MIDI ports and can be supported. 
Testers needed.

### Bass Floor POD

**Bass Floor POD** device does not have MIDI ports and cannot be supported.

### POD HD family

**POD HD** family consists of many devices (**POD HD Desktop**, **POD
HD Pro**, **POD Pro X**, **POD HD 300/400/500/500X**) and does have
MIDI ports. These devices come with a separate edit software, POD HD Edit.
Whether the POD HD devices communication is anyhow similar to that of
POD/PODxt ans whether these devices can be supported is unknown.

### HD147

**HD147** is an amp head that has much in common with the POD modelling
amps including the MIDI communication protocols. This device is likely
possible to support in pod-ui. Testers needed.

### <a name=other></a> Vetta II, Flextone III

**Vetta II** family of devices and **Flextone III** family of device 
are amps/heads that have much in common with the POD modelling amps
including the MIDI communication protocols. These devices are likely
possible to support in pod-ui. Testers needed.

### DT-series

**DT25** and **DT50** series devices could possibly be supported in
pod-ui much like the rest of the POD legacy products. 

Check out [DT Edit](http://rome2.github.io/dtedit/) software for a
modern control software for DT-series devices.

