#!/usr/bin/env bash

set -euo pipefail

if [ -z ${EDITION+x} ]; then
    EDITION=$1
fi

source ./buildscripts/common.sh
echo "Building $EDITION"
echo "FEATURES: $EDITION_FEATURES"

PKG_CONFIG_ALLOW_CROSS=1 OPENSSL_INCLUDE_DIR="/usr/include -I/usr/include/x86_64-linux-gnu" OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu cargo build --release --features "$EDITION_FEATURES" --target x86_64-unknown-linux-musl