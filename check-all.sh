#!/usr/bin/env bash

set -e pipefail

SETTINGS=$(yq ea "." editions.yaml | yq 'keys[]')
IFS=$'\n'
while IFS=$'\n' read -r EDITION; do
    ./check-edition.sh $EDITION
done <<< "$SETTINGS"
