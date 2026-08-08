#!/usr/bin/env bash

set -euo pipefail

if [ -z ${EDITION+x} ]; then
    EDITION=$1
fi

# comma seperated list of features for the specified version
source ./buildscripts/common.sh
echo FEATURES:
echo $EDITION_FEATURES

cargo build --release --features "$EDITION_FEATURES"
