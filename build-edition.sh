#!/usr/bin/env bash

set -euo pipefail

if [ -z ${EDITION+x} ]; then
    EDITION=$1
fi

# comma seperated list of features for the specified version
source ./buildscripts/common.sh
echo building $EDITION
echo FEATURES:
echo $EDITION_FEATURES

if [[ -v RNEX_STATIC ]]; then
    cargo build --release --features "$EDITION_FEATURES"
else
    OPENSSL_LIB_DIR=/usr/lib OPENSSL_INCLUDE_DIR=/usr/include/openssl OPENSSL_STATIC=1 RUSTFLAGS="-C relocation-model=static -C linker=ld.lld" cargo build --release --features "$EDITION_FEATURES" --target x86_64-unknown-linux-musl
fi
