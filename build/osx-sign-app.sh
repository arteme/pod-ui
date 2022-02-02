# usage: osx-sign.sh <path>
set -e
source .codesign

echo "Signing with '$IDENTITY'"

F=$(mktemp -d)

sign_app()
{
  codesign --verbose --force --deep --strict --options runtime --timestamp \
	   --entitlements ./build/osx/pod-ui.entitlements \
           --sign "$IDENTITY" "$1"
}

find $1 -path '*MacOS*' -perm -o+x -type f > $F/files
find $1 -name '*.dylib' -o -name '*.so' >> $F/files
APP=$(find $1 -name '*.app' -type d)

while read -r line; do
    sign_app "$line"
done < $F/files

sign_app "$APP"

codesign --verify -vvvv "$APP"
rm -rf $F
