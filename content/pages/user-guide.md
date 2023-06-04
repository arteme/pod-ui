title: User Guide


![application UI]({static}/images/pod-ui-v0.7.0.png)

[TOC]

# The UI

The application UI is rather straight-forward and functional. Unlike
**Line6 Edit** and popular guitar processors, it does not borrow from
the "knobs and switches" theme of guitar gadgets. Instead, it is a
clean, if conservative, all-in-your-face UI.

The UI has a title bar, a program list and the actual device controls.  

## The application titlebar

![titlebar]({static}/images/titlebar.png)

The titlebar gives you information about connected device, the state of
the app and offers several buttons, as well as traditional window
controls.

The titlebar allows you to see at a glance the application version and
device connection information &mdash; device version, input/output port and
if the device is connected using as a different model device.

![program list button]({static}/images/program-list-button.png)
If the connected device has more than 36 user-editable programs
(POD 2.0 has 36 user-editable programs, PocketPOD has 124), then a
"show all programs" toggle button is shown in the titlebar. It toggles
paged programs list (36 programs per page) view and full programs list
(all programs at once) view.
{.img-l-100px}

![transfer indicator]({static}/images/transfer-indicator.png)
When MIDI messages are sent or received by the application the MIDI
transfer indicator appears in the titlebar. The upwards triangle
&#9650; symbol denotes a message that the application sends to the
MIDI output port and the downwards triangle &#9660; symbol denotes a
message that the application receives from the MIDI input port. The
indicator is shown for 500ms for every message seen.
{.img-l-100px}

![panic indicator]({static}/images/panic-indicator.png)
When an internal application thread crashes, this indicator will
appear. Once this happens, the app may not receive, send or process
MIDI messages and thus should be restarted. If the application is
downloaded from the official GitHub releases page, it will have
been compiled with [sentry](https://getsentry.com) support for 
reporting errors and crashes. In this case, an error report will
have been sent to the developers.
{.img-l-100px}

![settings button]({static}/images/settings-button.png)
Click this button to open the settings dialog to change any of the
application settings, such as MIDI input/output port, device model
or MIDI channel used to communicate with the connected device (not
applicable to all device models). 
{.img-l-100px}


## The program list

The program list may look different for different devices. Below
is the program list of the same device as a POD 2.0 (left) and
as Pocket POD (right).

![program list (pod 2.0)]({static}/images/program-list-pod2.0.png)
![program list (pocketpod)]({static}/images/program-list-pocketpod.png)
{.flex-space}

The button configuration may be different. The "Manual" button used
to select the manual mode is only shown if the device supports it
(POD 2.0 does, PocketPOD doesn't). If the device has more than
36 programs, the buttons to switch the current program list page
([ **<** ] and [ **>** ]) are shown when the program list is in the
paged view mode. Toggling the "show all programs" view shows all
programs and hides the program list page navigation controls.

When a program button is clicked, the device will switch to that program.
When the program is changed on the device, it will also switch in the
application. If the program switches to a program on a page not currently
show, the program list will switch to that page.

### Edited programs

![program list edited]({static}/images/program-list-edited.png)
When the program is edited (some controls have been changed),
the program is marked as *edited*, which is shown as bold-italic
style on the program button in the program list.
<br/><br/>
When the program is saved to the device or loaded from it, the edited
status will be cleared.
{.img-l-300px .clearfix}

### Loading and storing programs

User-editable programs (otherwise known as patches, or presets) stored in
the device can be loaded into the application UI and edited. Some devices,
such as PocketPOD, also contain non-editable programs that can be saved
into user-editable program locations. **pod-ui** does not deal with
non-editable programs.

Program editing on the device works as follows. When program is switched on
the device, it is loaded into the edit buffer or that device so that the
user can edit it by adjusting the controls. If the program is not explicitly
saved, when the program is switched again, the new program will be loaded
into the edit buffer and all previous changes will be lost.

The application, however, keeps track of all the changes made to the
loaded programs. They are marked as modified and when the current program
is switched to a modified one, this modified program is sent to the
device.  

![program controls]({static}/images/program-controls.png)
Below the programs list you will find the controls to load programs
from the device and to store them to the device.
<br/><br/>
The following actions are available:
{.img-l-300px .clearfix}

- **Load** &mdash; load the edit buffer from the device to the application.
  This will modify the currently selected program in the application;

- **Load Patch** &mdash; load the selected program from the device
  into the application. This will load the selected program as it is stored
  in the device and will overwrite any modifications done to the program
  in the application, thus clearing the "modified" status;

- **Load All** &mdash; load all programs from the device into the
  application overwriting any modifications done in the application and
  resetting their modified statues;

- **Store** &mdash; send the edit buffer from the application to the device.
  This does not modify any stored programs, only affecting the edit buffer;

- **Store Patch** &mdash; store the selected program from the application
  into the device at the selected program location. The "modified" program
  status in the UI will be reset;

- **Load All** &mdash; store all programs from the application into the
  device resetting their modified statues;

## The controls

![device controls]({static}/images/device-controls.png)
{.flex-space}

Here you can change all controls on your modelling amp that can possibly
be changed using MIDI. These are typically more than can be changed using
the knobs on the device itself. Any change made here is sent as s MIDI CC
(control change) message to the device and any knob turned on the device
similarly sends a MIDI CC message to the application.

Some controls are hidden based on the selected amp model or effect. Some
controls are disabled based on the state of current toggles.

Different modelling amp models have different adjustable controls so the
UI may look different depending on the selected device model.

# Settings

~todo~

# Command-line

~todo~
