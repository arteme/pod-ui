#!/bin/bash

set -xe

build_debug_osx() {
    export RUSTFLAGS="-C split-debuginfo=packed"
    SENTRY=1 cargo build
    SIGN=1 ./build/mk-osx-dist.sh
    ./build/sentry-upload-dsyms.sh
}

build_debug_linux() {
    if [[ -n "$DOCKER_POD_UI_BUILD" ]];
    then
        SENTRY=1 cargo build
        ./build/mk-appimage-dist.sh
    else
        ./build/mk-appimage-dist-docker.sh
    fi
}

build_debug_win64() {
    SENTRY=1 cargo build
    bash ./build/mk-win64-dist.sh

    SENTRY=1 cargo build -F "winrt"
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

