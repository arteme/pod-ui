# https://gist.github.com/mjakeman/0add69647a048a5e262d591072c7facb
# maybe also add win10 theme? https://www.gtk.org/docs/installations/windows

V=$(git describe --tags --always --dirty)
N=pod-ui-$V-win64
DIST=debug
DIR=target/$N
TOOLS_DIR=$(dirname $0)

rm -rf $DIR
rm -rf "target/$N.zip"

mkdir -p $DIR

$TOOLS_DIR/collect-gtk.sh $DIR

cp target/$DIST/*.exe $DIR

echo "Copying dlls..."
ldd target/$DIST/*.exe | grep '/mingw.*/.*\.dll' -o | xargs -I{} cp '{}' $DIR

LOADERS=$(find $DIR/lib/gdk-pixbuf-2.0 -name '*.dll')
ldd $LOADERS | grep '/mingw.*/.*\.dll' -o | xargs -I{} cp '{}' $DIR

cd $DIR
cd ..
zip -r $N.zip $N

echo "!!! $DIR"
ls -sh $N.zip
