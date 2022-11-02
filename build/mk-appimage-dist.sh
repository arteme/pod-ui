V=$(git describe --tags --always --match 'v*' --dirty)
N=pod-ui-$V
DIST=debug
DIR=target/appdir
TOOLS_DIR=$(dirname $0)
LINUXDEPLOY=~/apps/appimage/linuxdeploy-x86_64.AppImage

rm -rf $DIR

mkdir -p $DIR/usr/bin
cp target/$DIST/pod-gui $DIR/usr/bin
sed "s|@VERSION@|$V|;s|@EXEC@|pod-gui|" \
       < build/linux/pod-ui.desktop.in > target/pod-ui.desktop
cp gui/resources/icon.png target/pod-ui.png

./build/collect-gtk.sh $DIR/usr

mkdir -p $DIR/apprun-hooks
cp ./build/linux/linuxdeploy-plugin-gtk.sh $DIR/apprun-hooks

pushd target 

export VERSION=$V

# make appimage
$LINUXDEPLOY --appdir ../$DIR \
	--library /usr/lib/libharfbuzz.so.0 \
	--library /usr/lib/libgtk-3.so.0 \
	--library /usr/lib/libgio-2.0.so.0 \
	--library /usr/lib/libgobject-2.0.so.0 \
	--library /usr/lib/libpango-1.0.so.0 \
	--library /usr/lib/libpangocairo-1.0.so.0 \
	--library /usr/lib/libpangoft2-1.0.so.0 \
	--desktop-file pod-ui.desktop --icon-file pod-ui.png \
       	--output appimage

popd

echo "!!! $DIR"
ls -sh target/*.AppImage | grep "$V"
