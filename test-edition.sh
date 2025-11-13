#!/usr/bin/env bash
if [ -z ${EDITION+x} ]; then
    EDITION=$1
fi

# comma seperated list of features for the specified version
source ./buildscripts/common.sh
echo FEATURES:
echo $EDITION_FEATURES

OPENSSL_LIB_DIR=/usr/lib OPENSSL_INCLUDE_DIR=/usr/include/openssl OPENSSL_STATIC=1 RUSTFLAGS="-C relocation-model=static -C linker=ld.lld" cargo test --features "$EDITION_FEATURES" --target x86_64-unknown-linux-musl