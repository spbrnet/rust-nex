#!/usr/bin/env bash

set -euo pipefail

if [ -z ${EDITION+x} ]; then
    EDITION=$1
fi

source ./buildscripts/common.sh
echo building $EDITION
echo FEATURES:
echo $EDITION_FEATURES

if [[ ! -v RNEX_STATIC ]]; then
    cargo build --release --features "$EDITION_FEATURES"
else
    CC_x86_64_unknown_linux_musl=musl-gcc OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include OPENSSL_STATIC=1 RUSTFLAGS="-C relocation-model=static -C linker=ld.lld" cargo build --release --features "$EDITION_FEATURES" --target x86_64-unknown-linux-musl
fi