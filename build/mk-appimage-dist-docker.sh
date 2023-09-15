#!/bin/bash
#
# This requires an "appimage build base" docker images built:
#   docker build -t pod-ui-appimage-build-base -f build/linux/Dockerfile.build-base build/linux
#   docker build -t pod-ui-appimage-build-base-debian10 -f build/linux/Dockerfile.build-base.debian10 build/linux
#
#

V=$(git describe --tags --always --match 'v*' --dirty)
N=pod-ui-$V
DIST=debug

docker run -it --rm \
	--user "$(id -u)":"$(id -g)" \
	-v `pwd`:/build -w /build \
	-v ~/.cargo:/.cargo -e CARGO_HOME=/.cargo \
    -e CARGO_TAGET_DIR=target.docker$1 \
	--device /dev/fuse --cap-add SYS_ADMIN \
       	pod-ui-appimage-build-base$1:latest /bin/bash -l ./build/release.sh $1

echo "!!! $DIR"
find target.docker$1/ -name '*.AppImage' -exec ls -sh \{} \; | grep "$V"
