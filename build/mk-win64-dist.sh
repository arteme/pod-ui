# https://gist.github.com/mjakeman/0add69647a048a5e262d591072c7facb
# maybe also add win10 theme? https://www.gtk.org/docs/installations/windows

DIST=debug
DIR=target/pod-ui-$(git describe --tags --always --dirty)-win64
THEMES_SRC_DIR=../themes
ICONS_SRC_DIR=../icons/paper-icon-theme-master

GTK_THEME="B00merang-Flat"
ICON_THEME="Paper"
ICONS=(
 # gtk 
 # collected from gtk 3.24 source tree:
 #   grep -R '--include=*.ui' '--include=*.c' --exclude-dir=demos \
 #        --exclude-dir=testsuite --exclude-dir=examples --exclude-dir=docs \
 #        -E '[">][a-z-]+-symbolic["<]' -oh . | tr -d '"<>' | sort -u
 applications-science-symbolic application-x-executable-symbolic
 audio-input-microphone-symbolic audio-volume-high-symbolic audio-volume-low-symbolic
 audio-volume-medium-symbolic audio-volume-muted-symbolic bluetooth-active-symbolic
 bookmark-new-symbolic camera-web-symbolic caps-lock-symbolic changes-allow-symbolic
 changes-prevent-symbolic color-select-symbolic dialog-information-symbolic
 dialog-password-symbolic dialog-question-symbolic dialog-warning-symbolic
 document-new-symbolic document-open-recent-symbolic document-open-symbolic
 document-save-symbolic drive-harddisk-symbolic edit-clear-symbolic edit-copy-symbolic
 edit-cut-symbolic edit-find-symbolic edit-paste-symbolic edit-redo-symbolic
 edit-select-all-symbolic edit-undo-symbolic emblem-synchronizing-symbolic
 emblem-system-symbolic emoji-activities-symbolic emoji-body-symbolic
 emoji-flags-symbolic emoji-food-symbolic emoji-nature-symbolic emoji-objects-symbolic
 emoji-people-symbolic emoji-recent-symbolic emoji-symbols-symbolic
 emoji-travel-symbolic face-smile-symbolic find-location-symbolic
 folder-documents-symbolic folder-download-symbolic folder-music-symbolic
 folder-new-symbolic folder-pictures-symbolic folder-publicshare-symbolic
 folder-remote-symbolic folder-saved-search-symbolic folder-symbolic
 folder-templates-symbolic folder-videos-symbolic format-text-italic-symbolic
 format-text-strikethrough-symbolic format-text-underline-symbolic
 gesture-pinch-symbolic gesture-rotate-anticlockwise-symbolic
 gesture-rotate-clockwise-symbolic gesture-stretch-symbolic
 gesture-two-finger-swipe-left-symbolic gesture-two-finger-swipe-right-symbolic
 go-down-symbolic go-next-symbolic go-previous-symbolic go-up-symbolic
 input-gaming-symbolic list-add-symbolic list-remove-symbolic media-eject-symbolic
 media-optical-symbolic media-playback-pause-symbolic media-record-symbolic
 network-server-symbolic network-workgroup-symbolic object-select-symbolic
 open-menu-symbolic pan-down-symbolic pan-end-symbolic pan-start-symbolic
 pan-up-symbolic preferences-desktop-locale-symbolic starred-symbolic
 start-here-symbolic switch-off-symbolic switch-on-symbolic text-x-generic-symbolic
 user-desktop-symbolic user-home-symbolic user-trash-full-symbolic user-trash-symbolic
 use-symbolic view-grid-symbolic view-list-symbolic view-refresh-symbolic
 window-close-symbolic window-maximize-symbolic window-minimize-symbolic
 window-restore-symbolic
 # app ui
 preferences-system-symbolic open-menu-symbolic
 dialog-ok dialog-error dialog-warning
)

mkdir -p $DIR
cp target/$DIST/*.exe $DIR

echo "1. libs"
ldd target/$DIST/*.exe | grep '/mingw.*/.*\.dll' -o | xargs -I{} cp '{}' $DIR

echo "2. gdkpixbuf loaders"
mkdir -p $DIR/lib
cp -r /mingw64/lib/gdk-pixbuf-2.0 $DIR/lib/gdk-pixbuf-2.0
find $DIR/lib/gdk-pixbuf-2.0 -name '*.a' -delete
dlls=$(find $DIR/lib/gdk-pixbuf-2.0 -name '*.dll')
ldd $dlls | grep '/mingw.*/.*\.dll' -o | xargs -I{} cp '{}' $DIR

echo "3. theme"
mkdir -p $DIR/share/themes
cp -r $THEMES_SRC_DIR/$GTK_THEME $DIR/share/themes/

echo "4. icons"
ICONS=$(IFS="|"; echo "${ICONS[*]}")
mkdir -p $DIR/share/icons/$ICON_THEME
T=$(realpath $DIR/share/icons/$ICON_THEME)
cp $ICONS_SRC_DIR/$ICON_THEME/index.theme $T
(cd $ICONS_SRC_DIR/$ICON_THEME; 
 find . -type f -regextype egrep -regex ".*/($ICONS)\..*" | cpio -pdm $T)
gtk-update-icon-cache-3.0 $T

echo "5. settings"
mkdir -p $DIR/etc/gtk-3.0
cat > $DIR/etc/gtk-3.0/settings.ini <<EOF
[Settings]
gtk-theme-name = $GTK_THEME
gtk-icon-theme-name = $ICON_THEME
EOF

echo "6. schemas"
mkdir -p $DIR/share/glib-2.0/schemas
cp /mingw64/share/glib-2.0/schemas/* $DIR/share/glib-2.0/schemas/
glib-compile-schemas $DIR/share/glib-2.0/schemas/

echo "!!! $DIR !!!"
