#!/usr/bin/env bash

set -euo pipefail

if [ -z ${EDITION+x} ]; then
    EDITION=$1
fi

# comma seperated list of features for the specified version
source ./buildscripts/common.sh
echo FEATURES:
echo $EDITION_FEATURES
echo ENV SETTINGS:
env

OPENSSL_VENDORED=1 cargo test --features "$EDITION_FEATURES" --target x86_64-unknown-linux-musl
