#! /usr/bin/env bash

gsettings get org.gnome.desktop.interface gtk-theme 2> /dev/null | grep -qi "dark" && GTK_THEME_VARIANT="dark" || GTK_THEME_VARIANT="light"
APPIMAGE_GTK_THEME="${APPIMAGE_GTK_THEME:-"Adwaita:$GTK_THEME_VARIANT"}" # Allow user to override theme (discouraged)

# in case we run from an AppImage, we use the $APPDIR environment variable as a template for the temporary directory that should be created
# this allows users to attribute the tempdir to the running AppImage
if [ "$APPDIR" != "" ]; then
    tempdir_template="$APPDIR".ld-p-gtk-tmp-XXXXXX
else
    tempdir_template=/tmp/.ld-p-gtk-tmp-XXXXXX
fi

export CACHEDIR="$(mktemp -d "$tempdir_template")"

export APPDIR="${APPDIR:-"$(dirname "$(realpath "$0")")"}" # Workaround to run extracted AppImage
export GTK_DATA_PREFIX="$APPDIR"

export GDK_BACKEND=x11 # Crash with Wayland backend on Wayland
export XDG_DATA_DIRS="$APPDIR/usr/share:/usr/share:$XDG_DATA_DIRS" # g_get_system_data_dirs() from GLib
export GSETTINGS_SCHEMA_DIR="$APPDIR/usr/share/glib-2.0/schemas"
export GTK_EXE_PREFIX="$APPDIR/usr"
export GTK_PATH="$APPDIR/usr/lib/gtk-3.0/modules"
export GTK_IM_MODULE_DIR="$APPDIR/usr/lib/gtk-3.0/3.0.0/immodules"
export GTK_IM_MODULE_FILE="$CACHEDIR/immodules.cache"
sed "s|@LIBDIR@/gtk-3.0|$APPDIR/usr/lib/gtk-3.0|g" "$APPDIR/usr/lib/gtk-3.0/3.0.0/immodules.cache" > "$GTK_IM_MODULE_FILE"
export GDK_PIXBUF_MODULEDIR="$APPDIR/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders"
export GDK_PIXBUF_MODULE_FILE="$CACHEDIR/loaders.cache"
sed "s|@LIBDIR@/gdk-pixbuf-2.0/2.10.0/loaders|$APPDIR/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders|g" "$APPDIR/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache" > "$GDK_PIXBUF_MODULE_FILE"
export XDG_CONFIG_DIRS="$APPDIR/usr/etc:$XDG_CONFIG_DIRS"

# a hackish way to add aplication-provided scaled icons as
# fallback for whatever the platform doesn't provide
base=usr/share/icons/Paper/scalable
export GTK_ADD_ICON_PATH="$APPDIR/$base/actions:$APPDIR/$base/emblems"
