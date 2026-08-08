#!/usr/bin/env bash
set -e

export EDITION=$1
source /etc/environment 2>/dev/null || true
: "${RNEX_CONTAINER_PLATFORM:=docker}"

export SHORT_SHA=${GITHUB_SHA::6}
export CI_COMMIT_SHORT_SHA=${CI_COMMIT_SHORT_SHA:-$SHORT_SHA}

echo "building $EDITION"
export RNEX_STATIC=1
export TARGET_DIR="target/x86_64-unknown-linux-musl/release"

./test-edition.sh
./build-edition.sh

mkdir -p dist/
cp $TARGET_DIR/rnex-server-backend-node-holder dist/
cp $TARGET_DIR/proxy_insecure dist/
cp $TARGET_DIR/proxy_secure dist/
cp $TARGET_DIR/rnex-server-backend-auth dist/
cp $TARGET_DIR/rnex-server-backend-secure dist/

TARGETS=("node-holder" "proxy-secure" "proxy-insecure" "backend-auth" "backend-secure")

for TARGET in "${TARGETS[@]}"; do
    $RNEX_CONTAINER_PLATFORM build \
        --target="$TARGET" \
        -t "$CI_REGISTRY_IMAGE/$EDITION/$TARGET:$CI_COMMIT_SHORT_SHA" .
    
    $RNEX_CONTAINER_PLATFORM push "$CI_REGISTRY_IMAGE/$EDITION/$TARGET:$CI_COMMIT_SHORT_SHA"
done