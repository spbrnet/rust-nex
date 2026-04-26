#!/usr/bin/env bash

set -euo pipefail

EDITIONS=$(yq ea "." editions.yaml | yq 'keys[]')
IFS=$'\n'
while IFS=$'\n' read -r EDITION; do
    if [[ $(yq ea ".$EDITION.include-in-checkall" editions.yaml) == "true" ]]
    then
        export EDITION
        ./check-edition.sh $EDITION
    fi
done <<< "$EDITIONS"
