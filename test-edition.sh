#!/usr/bin/env bash

set -euo pipefail

if [ -z ${EDITION+x} ]; then
    EDITION=$1
fi

# comma seperated list of features for the specified version
source ./buildscripts/common.sh
echo FEATURES:
echo $EDITION_FEATURES

PKG_CONFIG_ALLOW_CROSS=1 OPENSSL_DIR=/usr cargo build --release --features "$EDITION_FEATURES" --target x86_64-unknown-linux-musl
