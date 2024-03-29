#!/bin/bash

set -e

EXTRA=$1

V=$(git describe --tags --always --match 'v*' --dirty)
N=pod-ui-$V
T=${CARGO_TARGET_DIR:-target}
DIST=release
DIR=$T/appdir
TOOLS_DIR=$(dirname $0)
LINUXDEPLOY=../build/linux/linuxdeploy-x86_64.AppImage

rm -rf $DIR

mkdir -p $DIR/usr/bin
cp $T/$DIST/pod-gui $DIR/usr/bin
sed "s|@VERSION@|$V|;s|@EXEC@|pod-gui|" \
       < build/linux/pod-ui.desktop.in > $T/pod-ui.desktop
cp gui/resources/icon.png $T/pod-ui.png

./build/collect-gtk.sh $DIR/usr

LIBDIR=$(pkg-config --variable=libdir gtk+-3.0)
mkdir -p $DIR/apprun-hooks
sed "s|@LIBDIR@|$LIBDIR|g" < ./build/linux/linuxdeploy-plugin-gtk.sh > $DIR/apprun-hooks/linuxdeploy-plugin-gtk.sh

pushd $T 

export VERSION=$V$EXTRA

# make linuxdeploy & appimage  happy
export -n SIGN # no linuxdeploy, no signing!
export LINUXDEPLOY_OUTPUT_VERSION=$VERSION

# make appimage
$LINUXDEPLOY --appdir ../$DIR \
	--library $LIBDIR/libharfbuzz.so.0 \
	--library $LIBDIR/libgtk-3.so.0 \
	--library $LIBDIR/libgio-2.0.so.0 \
	--library $LIBDIR/libgobject-2.0.so.0 \
	--library $LIBDIR/libpango-1.0.so.0 \
	--library $LIBDIR/libpangocairo-1.0.so.0 \
	--library $LIBDIR/libpangoft2-1.0.so.0 \
	--desktop-file pod-ui.desktop --icon-file pod-ui.png \
       	--output appimage

popd

echo "!!! $DIR"
find $T/ -name '*.AppImage' -exec ls -sh \{} \; | grep "$V"
