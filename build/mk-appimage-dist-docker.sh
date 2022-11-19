#!/bin/bash
#
# This requires an "appimage build base" docker image built:
#   docker build -t pod-ui-appimage-build-base -f build/linux/Dockerfile.build-base build/linux
#
#

V=$(git describe --tags --always --match 'v*' --dirty)
N=pod-ui-$V
DIST=debug

docker run -it \
	--user "$(id -u)":"$(id -g)" \
	-v `pwd`:/build -w /build \
	-v ~/.cargo:/.cargo -e CARGO_HOME=/.cargo \
	--device /dev/fuse --cap-add SYS_ADMIN \
       	pod-ui-appimage-build-base:latest /bin/bash -l ./build/release.sh

echo "!!! $DIR"
ls -sh target/*.AppImage | grep "$V"
