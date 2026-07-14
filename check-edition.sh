#!/usr/bin/env bash


set -euo pipefail

if [ -z ${EDITION+x} ]; then
    export EDITION=$1
fi

# comma seperated list of features for the specified version
source ./buildscripts/common.sh
echo CHECKING $EDITION
echo FEATURES:
echo $EDITION_FEATURES

# RUSTFLAGS="--deny warnings" cargo clippy --workspace --features "$EDITION_FEATURES"
# RUSTFLAGS="--deny warnings" cargo check --workspace --features "$EDITION_FEATURES"
echo "edition checks are disabled right now due to being in"
