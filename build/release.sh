#!/bin/bash

set -xe

export SENTRY=1
export RELEASE_CHECK=1
export SIGN=1

build_release_osx() {
    export RUSTFLAGS="-C split-debuginfo=packed"
    export RELEASE_PLATFORM="osx"
    cargo build --release
    ./build/mk-osx-dist.sh
    ./build/sentry-upload-dsyms.sh target/release/pod-gui{,.dSYM}
}

build_release_linux_real() {
    export RELEASE_PLATFORM="linux"
    cargo build --release
    ./build/linux-split-debuginfo.sh target/release/pod-gui
    ./build/mk-appimage-dist.sh
    ./build/sentry-upload-dsyms.sh target/release/pod-gui{,.debug}
}

build_release_linux() {
    if [[ -n "$DOCKER_POD_UI_BUILD" ]];
    then
        build_release_linux_real
    else
        ./build/mk-appimage-dist-docker.sh
    fi
}

build_release_win64() {
    export RUSTFLAGS="-C link-arg=-Wl,--build-id"
    export RELEASE_PLATFORM="win64$1"
    cargo build --release $2
    cp target/release/pod-gui.exe{,.full}
    ./build/linux-split-debuginfo.sh target/release/pod-gui.exe
    ./build/mk-win64-dist.sh $1
    ./build/sentry-upload-dsyms.sh target/release/pod-gui.exe.full
}

case "$(uname)" in
    Darwin)
        build_release_osx
        ;;
    Linux)
        build_release_linux
        ;;
    MINGW64*)
        build_release_win64
        build_release_win64 "-winrt" "-F winrt"
        ;;
    *)
        echo "Unsupposted platform"
        exit 1
esac
# vim:ts=4:sw=4:et:
