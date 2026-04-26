#!/usr/bin/env bash

export EDITION=$1
export BA="--network=host --build-arg EDITION=$1 --build-arg DATABASE_URL="$DATABASE_URL""

# podman build $BA -t "$CI_REGISTRY_IMAGE/$EDITION/dev-container:latest" --target=dev-container .
# podman push "$CI_REGISTRY_IMAGE/$EDITION/dev-container:latest"
podman build $BA -t "$CI_REGISTRY_IMAGE/$EDITION/node-holder:$CI_COMMIT_SHORT_SHA" --target=node-holder .
podman build $BA -t "$CI_REGISTRY_IMAGE/$EDITION/proxy-secure:$CI_COMMIT_SHORT_SHA" --target=proxy-secure .
podman build $BA -t "$CI_REGISTRY_IMAGE/$EDITION/proxy-insecure:$CI_COMMIT_SHORT_SHA" --target=proxy-insecure .
podman build $BA -t "$CI_REGISTRY_IMAGE/$EDITION/backend-auth:$CI_COMMIT_SHORT_SHA" --target=backend-auth .
podman build $BA -t "$CI_REGISTRY_IMAGE/$EDITION/backend-secure:$CI_COMMIT_SHORT_SHA" --target=backend-secure .
podman push "$CI_REGISTRY_IMAGE/$EDITION/node-holder:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/proxy-secure:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/proxy-insecure:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/backend-auth:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/backend-secure:$CI_COMMIT_SHORT_SHA"
