#!/usr/bin/env bash

set -euo pipefail

TMP_FEATURES_TRAILINGCOMMA=$(yq ea ".$EDITION.features" editions.yaml  | sed 's/- //g' | tr '\n' ',')
export EDITION_FEATURES=${TMP_FEATURES_TRAILINGCOMMA::-1}
SETTINGS=$(yq ea ".$EDITION.settings" editions.yaml | yq 'keys[]')
IFS=$'\n'
while IFS=$'\n' read -r KEY; do
  VAL=$(yq ea ".$EDITION.settings.$KEY" editions.yaml)
  declare "$KEY=$VAL"
  export $KEY
done <<< "$SETTINGS"
