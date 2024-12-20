#!/bin/bash
#
# usage: collect-gtk.sh <dest dir>
#
SRC_DIR=$(dirname $0)/resources
THEMES_SRC_DIR=$SRC_DIR/themes
ICONS_SRC_DIR=$SRC_DIR/icons

DIR=$1

GTK_THEME="Arc-Darker-solid"
ICON_THEME="Paper"
ICONS=(
 # gtk 
 # collected from gtk 3.24 source tree:
 #   grep -R '--include=*.ui' '--include=*.c' --exclude-dir=demos \
 #        --exclude-dir=testsuite --exclude-dir=examples --exclude-dir=docs \
 #        -E '[">][a-z-]+-symbolic["<]' -oh . | tr -d '"<>' | sort -u
 applications-science-symbolic application-x-executable-symbolic
 audio-input-microphone-symbolic audio-volume-high-symbolic
 audio-volume-low-symbolic audio-volume-medium-symbolic
 audio-volume-muted-symbolic bluetooth-active-symbolic bookmark-new-symbolic
 camera-web-symbolic caps-lock-symbolic changes-allow-symbolic
 changes-prevent-symbolic color-select-symbolic dialog-information-symbolic
 dialog-password-symbolic dialog-question-symbolic dialog-warning-symbolic
 document-new-symbolic document-open-recent-symbolic document-open-symbolic
 document-save-symbolic drive-harddisk-symbolic edit-clear-symbolic
 edit-copy-symbolic edit-cut-symbolic edit-find-symbolic edit-paste-symbolic
 edit-redo-symbolic edit-select-all-symbolic edit-undo-symbolic
 emblem-synchronizing-symbolic emblem-system-symbolic emoji-activities-symbolic
 emoji-body-symbolic emoji-flags-symbolic emoji-food-symbolic
 emoji-nature-symbolic emoji-objects-symbolic emoji-people-symbolic
 emoji-recent-symbolic emoji-symbols-symbolic emoji-travel-symbolic
 face-smile-symbolic find-location-symbolic folder-documents-symbolic
 folder-download-symbolic folder-music-symbolic folder-new-symbolic
 folder-pictures-symbolic folder-publicshare-symbolic folder-remote-symbolic
 folder-saved-search-symbolic folder-symbolic folder-templates-symbolic
 folder-videos-symbolic format-text-italic-symbolic
 format-text-strikethrough-symbolic format-text-underline-symbolic
 gesture-pinch-symbolic gesture-rotate-anticlockwise-symbolic
 gesture-rotate-clockwise-symbolic gesture-stretch-symbolic
 gesture-two-finger-swipe-left-symbolic gesture-two-finger-swipe-right-symbolic
 go-down-symbolic go-next-symbolic go-previous-symbolic go-up-symbolic
 input-gaming-symbolic list-add-symbolic list-remove-symbolic
 media-eject-symbolic media-optical-symbolic media-playback-pause-symbolic
 media-record-symbolic network-server-symbolic network-workgroup-symbolic
 object-select-symbolic open-menu-symbolic pan-down-symbolic pan-end-symbolic
 pan-start-symbolic pan-up-symbolic preferences-desktop-locale-symbolic
 starred-symbolic start-here-symbolic switch-off-symbolic switch-on-symbolic
 text-x-generic-symbolic user-desktop-symbolic user-home-symbolic
 user-trash-full-symbolic user-trash-symbolic use-symbolic view-grid-symbolic
 view-list-symbolic view-refresh-symbolic window-close-symbolic
 window-maximize-symbolic window-minimize-symbolic window-restore-symbolic
 # from theme:
 #   grep -Roh -- '-gtk-icontheme(.*)' . | grep -oh '(.*)' | tr -d "()\"'"
 selection-start-symbolic selection-end-symbolic process-working-symbolic
 # app ui
 emblem-important-symbolic application-menu-symbolic pane-show-symbolic
 dialog-ok dialog-error dialog-warning
)
EXCLUDE_PIXBUF=("*avif*" "*heif*" "*tif*" "*tga*" "*bmp*" "*ani*")

# not really the coreutils' realpath, but will do 
realpath() {
    cd $1 && pwd -P
}

echo "0. sanity check"
UPDATE_ICON_CACHE=$(which gtk3-update-icon-cache gtk-update-icon-cache-3.0 gtk-update-icon-cache 2>/dev/null | head -1)
echo "update-icon-cache: $UPDATE_ICON_CACHE"
ROOT=$(realpath $(dirname $UPDATE_ICON_CACHE)/..)
echo "root: $ROOT"
LIBDIR=$(pkg-config --variable=libdir gtk+-3.0)
# On MacOS with Homebrew every package installs its contents to a separate directory,
# then the files are linked in to /usr/local/... tree, so if we fo with gtk+-3.0 or
# gdk-pixbuf-2.0 libdir, we miss the other installed packages, like librsvg
[[ "`uname`" == "Darwin" ]] && LIBDIR="$ROOT/lib"
echo "libdir: $LIBDIR"

[[ -d "$THEMES_SRC_DIR" ]] && [[ -d "$ICONS_SRC_DIR" ]] || {
    echo "Resources missing!" >&2
    exit 1
}

echo "1. theme"
mkdir -p $DIR/share/themes
cp -r $THEMES_SRC_DIR/$GTK_THEME $DIR/share/themes/

echo "2. icons"
ICONS_PATTERN=$(IFS="|"; echo "${ICONS[*]}")
mkdir -p $DIR/share/icons/$ICON_THEME
T=$(realpath $DIR/share/icons/$ICON_THEME)
cp $ICONS_SRC_DIR/$ICON_THEME/index.theme $T
(cd $ICONS_SRC_DIR/$ICON_THEME;
 find . -type f | grep -E "/($ICONS_PATTERN)\." | cpio -pdm $T)
# Validate that we actually copied at least one image for every mentioned icon
(cd $T
 for i in "${ICONS[@]}"; do
   find . -name "$i.*" -print -quit | grep . -q
   [[ $? -eq 0 ]] || {
     echo "ICON NOT FOUND: $i"
   }
 done
)



$UPDATE_ICON_CACHE $T

echo "3. settings"
mkdir -p $DIR/etc/gtk-3.0
cat > $DIR/etc/gtk-3.0/settings.ini <<EOF
[Settings]
gtk-theme-name = $GTK_THEME
gtk-icon-theme-name = $ICON_THEME
EOF

echo "4. schemas"
mkdir -p $DIR/share/glib-2.0/schemas
cp $ROOT/share/glib-2.0/schemas/* $DIR/share/glib-2.0/schemas/
glib-compile-schemas $DIR/share/glib-2.0/schemas/

echo "5. gdkpixbuf loaders"
mkdir -p $DIR/lib
cp -RL $LIBDIR/gdk-pixbuf-2.0 $DIR/lib/
find $DIR/lib/gdk-pixbuf-2.0 -name '*.a' -delete

# remove excluded pixbuf loaders
args=()
for arg in "${EXCLUDE_PIXBUF[@]}"; do
	if [ "${#args}" -ne "0" ]; then
		args+=(-o)
	fi
	args+=(-name "$arg")
done
find $DIR/lib/gdk-pixbuf-2.0 \( "${args[@]}" \) -delete

if [ "$COLLECT_GTK_RELATIVE_PATHS" = "1" ]; then
	# update cache with relative paths
	CACHE=$(find $DIR/lib/gdk-pixbuf-2.0 -name 'loaders.cache')
	sed -i.old -E "s,\".*(lib/gdk-pixbuf.*)\",\"$2\\1\"," $CACHE
	rm $CACHE.old
fi

echo "6. immodules"
mkdir -p $DIR/lib/gtk-3.0/3.0.0
cp -RL $LIBDIR/gtk-3.0/3.0.0/immodules $DIR/lib/gtk-3.0/3.0.0
cp $LIBDIR/gtk-3.0/3.0.0/immodules.cache $DIR/lib/gtk-3.0/3.0.0

if [ "$COLLECT_GTK_RELATIVE_PATHS" = "1" ]; then
	# update cache with relative paths
	CACHE=$(find $DIR/lib/gtk-3.0 -name 'immodules.cache')
	sed -i.old -E "s,\".*(lib/gtk-3.0/.*)\",\"$2\\1\"," $CACHE
	rm $CACHE.old
fi
