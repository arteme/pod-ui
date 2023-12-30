#!/bin/bash

export SENTRY=0
export RELEASE_CHECK=0
export SIGN=0

`dirname $0`/release.sh
