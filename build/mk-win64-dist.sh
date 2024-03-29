# https://gist.github.com/mjakeman/0add69647a048a5e262d591072c7facb
# maybe also add win10 theme? https://www.gtk.org/docs/installations/windows

EXTRA=$1
T=${2:-target}

V=$(git describe --tags --always --match 'v*' --dirty)
N=pod-ui-$V-win64$EXTRA
DIST=release
DIR=$T/$N
TOOLS_DIR=$(dirname $0)

rm -rf $DIR
rm -rf "$T/$N.zip"

mkdir -p $DIR

COLLECT_GTK_RELATIVE_PATHS=1 $TOOLS_DIR/collect-gtk.sh $DIR

cp $T/$DIST/*.exe $DIR

echo "Copying dlls..."
ldd $T/$DIST/*.exe | grep '/mingw.*/.*\.dll' -o | xargs -I{} cp '{}' $DIR

LOADERS=$(find $DIR/lib/gdk-pixbuf-2.0 -name '*.dll')
ldd $LOADERS | grep '/mingw.*/.*\.dll' -o | xargs -I{} cp '{}' $DIR

cd $DIR
cd ..
zip -r $N.zip $N

echo "!!! $DIR"
ls -sh $N.zip
