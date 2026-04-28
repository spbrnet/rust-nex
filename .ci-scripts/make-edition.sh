#!/usr/bin/env bash
export EDITION=$1
source /etc/environment
: "${RNEX_CONTAINER_PLATFORM:=podman}"

for TARGET in node-holder proxy-secure proxy-insecure backend-auth backend-secure; do
    $RNEX_CONTAINER_PLATFORM build \
        --network=host \
        --build-arg EDITION="$EDITION" \
        --build-arg DATABASE_URL="$DATABASE_URL" \
        -t "$CI_REGISTRY_IMAGE/$EDITION/$TARGET:$CI_COMMIT_SHORT_SHA" \
        --target="$TARGET" .
    
    $RNEX_CONTAINER_PLATFORM push "$CI_REGISTRY_IMAGE/$EDITION/$TARGET:$CI_COMMIT_SHORT_SHA"
done