#!/bin/bash

set -xe
source .sentry
sentry-cli upload-dif --include-sources "$@"
