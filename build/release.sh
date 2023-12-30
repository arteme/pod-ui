#!/bin/bash

echo SENTRY=${SENTRY:-1}
echo RELEASE_CHECK=${RELEASE_CHECK:-1}
echo SIGN=${SIGN:-1}

export SENTRY RELEASE_CHECK SIGN

set -xe

build_release_osx() {
    export RUSTFLAGS="-C split-debuginfo=packed"
    export RELEASE_PLATFORM="osx"
    cargo build --release
    ./build/mk-osx-dist.sh
    [ $SENTRY == "1" ] && \
      ./build/sentry-upload-dsyms.sh target/release/pod-gui{,.dSYM}
}

build_release_linux_real() {
    T=target.docker$1
    export CARGO_TARGET_DIR=$T
    export RELEASE_PLATFORM="linux$1"
    cargo build --release
    ./build/linux-split-debuginfo.sh $T/release/pod-gui
    ./build/mk-appimage-dist.sh $1
    [ $SENTRY == "1" ] && \
      ./build/sentry-upload-dsyms.sh $T/release/pod-gui{,.debug}
}

# When running inside a pod-ui docker build container, this will run one
# single instance of "real linux release build" and the exit! When NOT
# running inside a pod-ui docker build container, this does noting!
build_release_linux_in_docker() {
    if [[ -n "$DOCKER_POD_UI_BUILD" ]];
    then
        build_release_linux_real $1
        exit
    fi
}

build_release_linux() {
    ./build/mk-appimage-dist-docker.sh $1
}

build_release_win64() {
    export RUSTFLAGS="-C link-arg=-Wl,--build-id"
    export RELEASE_PLATFORM="win64$1"
    cargo build --release $2
    cp target/release/pod-gui.exe{,.full}
    ./build/linux-split-debuginfo.sh target/release/pod-gui.exe
    ./build/mk-win64-dist.sh $1
    [ $SENTRY == "1" ] && \
      ./build/sentry-upload-dsyms.sh target/release/pod-gui.exe.full
}

case "$(uname)" in
    Darwin)
        build_release_osx
        ;;
    Linux)
        build_release_linux_in_docker $1

        build_release_linux
        build_release_linux "-debian10"
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
