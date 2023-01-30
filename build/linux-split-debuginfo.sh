#!/bin/bash
#
# see: https://github.com/rust-lang/rust/issues/34651
#
# ./build/linux-split-debuginfo.sh <path/file>
#

[[ -f "$1" ]] && [[ -r "$1" ]] || {
	echo "ERROR: not a readable file: $1" >&2
	return 1
}

cd `dirname $1`
F=`basename $1`

set -xe

objcopy --compress-debug-sections --only-keep-debug $F $F.debug
objcopy --strip-debug --add-gnu-debuglink=$F.debug $F
