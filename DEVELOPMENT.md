# Development

## Getting started

To build `pod-ui` you need to install `git`, the `rust` toolchain 
([rustup](https://rustup.rs/) is a popular tool to get you started), the
Gtk+ 3.x libraries. Additionally, platform-specific tools may be
required for making release packages.

### Linux

Refer to your distribution documentation on getting `git`, `rust` and
Gtk+ 3.x installed.

For packaging `pod-ui` AppImage you'll need
[linuxdeploy](https://github.com/linuxdeploy/linuxdeploy) and [linuxdeploy
gtk plugin](https://github.com/linuxdeploy/linuxdeploy-plugin-gtk):

```shell
mkdir -p ~/apps/appimage
cd ~/apps/appimage
wget -c "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
wget -c "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
```

**NOTE:** Both of these are continuous releases, which means that care must
be taken to ensure that `pod-ui` is compatible with updated linuxdeploy.

### Windows

On Windows you will need [MSYS2](https://www.msys2.org/wiki/MSYS2-installation/)
to compile `pod-ui`. It may also be possible to use Visual Studio and `rustup`,
or use `rustup` with MSYS2, but this has not been tested. Instead, we'll use
`rust` from MSYS2.

After installing MSYS2, open `MSYS2 Mingw x64` terminal from the Start menu:
```shell
pacman -S base_devel mingw-w64-x86_64-toolchain mingw-w64-x86_64-rust mingw-w64-x86_64-gtk3
```

### MacOS

On macOS, the easiest way to get the required tools and libraries installed
is using [Homebrew](https://brew.sh):

```shell
brew install rustup-init gtk+3 librsvg
rustup-init -y
source $HOME/.cargo/env
```

For packaging `pod-ui` you will need additional tools:
```shell
brew install dylibbundler akeru-inc/tap/xcnotary
```

## Building and running

The basic steps needed for building and running `pod-ui` is the same for
all platforms:

```shell
git clone --recurse-submodules git@github.com/arteme/pod-ui.git
cd pod-ui
cargo build
cargo run
```

The `--recurse-submodules` flag is not strictly needed for everyone,
since it also pulls the Gtk theme data needed for making distribution
packages.

## Packaging

Currently, packages can be built for Linux (AppImage), Windows and macOS.

Scripts needed to package `pod-ui` for distribution can be found
from `build/` directory.

`pod-ui` is built with Sentry support for uploading crash reports to
the cloud, but you need to specifically enable sentry at build-time
using `SENTRY=1` environment variable or `SENTRY_DSN=...` environment
variable at run-time.

### Windows

This will produce a zip-file for windows:

```shell
SENTRY=1 cargo build
bash ./build/mk-win64-dist.sh
```

### MacOS

The latest versions of MacOS require the developer to sign and notarize
the distributed packages so that they can be opened and ran on others'
systems. This requires developer keys for signing. Create the `.codesign`
file in the root of your `pod-ui` check-out containing the following: 

```shell
# security find-identity -v -p codesigning
IDENTITY="Developer ID Application: Xxxxx Xxxxx (XXXXXXXXXX)"
DEVELOPER="xxxxx@xxxxx.com"
DEVELOPER_KEY="notarization"
```

Now you can build, sign, notarize and staple the app:

```shell
SENTRY=1 cargo build
SIGN=1 ./build/mk-osx-dist.sh
```

This usually takes a few minutes as the `xcnotary` tool used for
notarizing the package will poll Apple's server for notarization
result for stapling.

Alternatively, one can omit `SIGN=1` and build unsigned package
of little usefulness.

## Linux

To build AppImage distribution packages do:

```shell
SENTRY=1 cargo build
./build/mk-appimage-dist.sh
```
