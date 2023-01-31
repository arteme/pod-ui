#!/bin/bash

set -xe

export SENTRY=1
export RELEASE_CHECK=1
export SIGN=1

build_debug_osx() {
    export RELEASE_PLATFORM="osx"
    cargo build --release
    ./build/mk-osx-dist.sh
}

build_debug_linux() {
    export RELEASE_PLATFORM="linux"
    if [[ -n "$DOCKER_POD_UI_BUILD" ]];
    then
        cargo build --release
	./build/linux-split-debuginfo.sh target/release/pod-gui
        ./build/mk-appimage-dist.sh
    else
        ./build/mk-appimage-dist-docker.sh
    fi
}

build_debug_win64() {
    export RELEASE_PLATFORM="win64"
    cargo build --release
    bash ./build/mk-win64-dist.sh

    export RELEASE_PLATFROM="win64-winrt"
    cargo build --release -F "winrt"
    bash ./build/mk-win64-dist.sh -winrt
}

case "$(uname)" in
    Darwin)
        build_debug_osx
        ;;
    Linux)
        build_debug_linux
        ;;
    MINGW64*)
	build_debug_win64
        ;;
    *)
        echo "Unsupposted platform"
        exit 1
esac

