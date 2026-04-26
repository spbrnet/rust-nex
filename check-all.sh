#!/usr/bin/env bash

SETTINGS=$(yq ea "." editions.yaml | yq 'keys[]')
IFS=$'\n'
while IFS=$'\n' read -r EDITION; do
    ./check-edition.sh $EDITION
done <<< "$SETTINGS"
