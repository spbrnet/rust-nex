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

cargo check --features "$EDITION_FEATURES"
