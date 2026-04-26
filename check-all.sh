#!/usr/bin/env bash

set -euo pipefail

SETTINGS=$(yq ea "." editions.yaml | yq 'keys[]')
IFS=$'\n'
while IFS=$'\n' read -r EDITION; do
    export EDITION
    ./check-edition.sh $EDITION
done <<< "$SETTINGS"
