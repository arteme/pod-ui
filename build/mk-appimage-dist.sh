V=$(git describe --tags --always --match 'v*' --dirty)
N=pod-ui-$V
DIST=debug
DIR=target/appdir
TOOLS_DIR=$(dirname $0)
LINUXDEPLOY=../build/linux/linuxdeploy-x86_64.AppImage

rm -rf $DIR

mkdir -p $DIR/usr/bin
cp target/$DIST/pod-gui $DIR/usr/bin
sed "s|@VERSION@|$V|;s|@EXEC@|pod-gui|" \
       < build/linux/pod-ui.desktop.in > target/pod-ui.desktop
cp gui/resources/icon.png target/pod-ui.png

./build/collect-gtk.sh $DIR/usr

LIBDIR=$(pkg-config --variable=libdir gtk+-3.0)
mkdir -p $DIR/apprun-hooks
sed "s|@LIBDIR@|$LIBDIR|g" < ./build/linux/linuxdeploy-plugin-gtk.sh > $DIR/apprun-hooks/linuxdeploy-plugin-gtk.sh

pushd target 

export VERSION=$V

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
ls -sh target/*.AppImage | grep "$V"
