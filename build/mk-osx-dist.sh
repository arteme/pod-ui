# http://bazaar.launchpad.net/~widelands-dev/widelands/trunk/view/head:/utils/macos/build_app.sh

DIST=debug
DIR=target/pod-ui-$(git describe --tags --always --dirty)-osx
THEMES_SRC_DIR=../themes
ICONS_SRC_DIR=../icons/paper-icon-theme-master

V=$(git describe --tags --always --dirty)

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

C=$DIR/Pod-UI.app/Contents
mkdir -p $DIR/Pod-UI.app/Contents/{Resources,MacOS}
cp gui/resources/icon.icns $C/Resources/pod-ui.icns
cat >$C/Info.plist <<EOF
{
  CFBundleName = pod-ui;
  CFBundleDisplayName = Pod-UI;
  CFBundleIdentifier = "io.github.arteme.pod-ui";
  CFBundleShortVersionString = "0.1.0";
  CFBundleVersion = "0.1.0.0";
  CFBundleInfoDictionaryVersion = "6.0";
  CFBundlePackageType = APPL;
  CFBundleSignatue = pdui;
  CFBundleExecutable = pod-gui;
  CFBundleIconFile = pod-ui.icns;
}
EOF

cp target/$DIST/pod-gui $C/MacOS

# Locate ASAN Library by asking llvm (nice trick by SirVer I suppose)
ASANLIB=$(echo "int main(void){return 0;}" |\
       	  xcrun clang -fsanitize=address -xc -o/dev/null -v - 2>&1 |\
       	  tr ' ' '\n' |\
	  grep libclang_rt.asan_osx_dynamic.dylib)
ASANPATH=`dirname $ASANLIB`

echo "Copying and fixing dynamic libraries... "
dylibbundler --create-dir --bundle-deps \
    --fix-file $C/MacOS/pod-gui \
    --dest-dir $C/libs \
    --search-path $ASANPATH

echo "Creating a DMG file..."
hdiutil create -fs HFS+ -volname "Pod-UI $V" -srcfolder $DIR "target/pod-ui-$V-osx.dmg"

echo "!!! $DIR !!!"
