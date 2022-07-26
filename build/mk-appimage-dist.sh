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

pushd target 

# first pass, deploy gtk things from `linuxdeploy-plugin-gtk`
export DEPLOY_GTK_VERSION=3
export VERSION=$V
$LINUXDEPLOY --appdir ../$DIR --plugin gtk \
	--desktop-file pod-ui.desktop --icon-file pod-ui.png

# fix linuxdeploy-plugin-gtk deployment
sed -i.old 's/export GTK_THEME=.*//' ../$DIR/apprun-hooks/linuxdeploy-plugin-gtk.sh
cat >>../$DIR/apprun-hooks/linuxdeploy-plugin-gtk.sh <<EOF
export XDG_CONFIG_DIRS="\$APPDIR/usr/etc:\$XDG_CONFIG_DIRS"
EOF
rm ../$DIR/apprun-hooks/*.old

# final pass, make appimage
$LINUXDEPLOY --appdir ../$DIR --output appimage

popd

echo "!!! $DIR"
ls -sh target/*.AppImage | grep "$V"
