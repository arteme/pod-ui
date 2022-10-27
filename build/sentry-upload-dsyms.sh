#!/bin/bash

set -xe
source .sentry
sentry-cli --auth-token $TOKEN upload-dif --org $ORG --project $PROJECT \
	target/debug/pod-gui.dSYM

