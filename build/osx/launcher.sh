#!/bin/sh

dir=$(dirname "$0")
cd "$dir"

dir=$(pwd)
base="$dir/.."
res="$base/Resources"

etc="$res/etc"
lib="$res/lib"
share="$res/share"

export XDG_CONFIG_DIRS="$etc"
export XDG_DATA_DIRS="$share"
export DYLD_LIBRARY_PATH="$lib"
export GTK_PATH="$res"
export GTK_DATA_PREFIX="$res"
export GTK_EXE_PREFIX="$res"
export GDK_PIXBUF_MODULE_FILE="$lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"

bin="$base/MacOS/pod-gui"

# Select dark theme variant if the UI is in dark mode
style=$(defaults read -g AppleInterfaceStyle 2>/dev/null)
if [ "$style" = "Dark" ]; then
  theme=$(cat "$etc/gtk-3.0/settings.ini" | grep gtk-theme-name |\
	  cut -d= -f2 | sed 's/^ *//;s/ *$//')
  export GTK_THEME="$theme:dark"
fi

#exec sudo -E dtruss "$bin" "$@"
exec "$bin" "$@"
