# usage: osx-sign.sh <path>
set -e
source .codesign

echo "Signing with '$IDENTITY'"

sign_dmg()
{
  codesign --verbose --force --sign "$IDENTITY" "$1"
}

sign_dmg "$1"

codesign --verify -vvvv "$1"
rm -rf $F
