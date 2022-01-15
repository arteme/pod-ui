# https://gist.github.com/mjakeman/0add69647a048a5e262d591072c7facb
# maybe also add win10 theme? https://www.gtk.org/docs/installations/windows

DIST=debug
DIR=target/dist

mkdir -p $DIR
cp target/$DIST/*.exe $DIR

# 1. collect libs
ldd target/$DIST/*.exe | grep '/mingw.*/.*\.dll' -o | xargs -I{} cp '{}' $DIR

# 2. gdkpixbuf loaders
mkdir -p $DIR/lib
cp -r /mingw64/lib/gdk-pixbuf-2.0 $DIR/lib/gdk-pixbuf-2.0

# 3. icons
mkdir -p $DIR/share/icons
cp -r /mingw64/share/icons/* $DIR/share/icons/

# 4. schemas
mkdir -p $DIR/share/glib-2.0/schemas
cp /mingw64/share/glib-2.0/schemas/* $DIR/share/glib-2.0/schemas/
glib-compile-schemas $DIR/share/glib-2.0/schemas/
