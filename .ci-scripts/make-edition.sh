#!/usr/bin/env bash

export EDITION=$1

podman build -t "$CI_REGISTRY_IMAGE/$EDITION/dev-container:latest" --target=dev-container .
podman push "$CI_REGISTRY_IMAGE/$EDITION/dev-container:latest"
podman build -t "$CI_REGISTRY_IMAGE/$EDITION/node-holder:$CI_COMMIT_SHORT_SHA" --target=node-holder .
podman build -t "$CI_REGISTRY_IMAGE/$EDITION/proxy-secure:$CI_COMMIT_SHORT_SHA" --target=proxy-secure .
podman build -t "$CI_REGISTRY_IMAGE/$EDITION/proxy-insecure:$CI_COMMIT_SHORT_SHA" --target=proxy-insecure .
podman build -t "$CI_REGISTRY_IMAGE/$EDITION/backend-auth:$CI_COMMIT_SHORT_SHA" --target=backend-auth .
podman build -t "$CI_REGISTRY_IMAGE/$EDITION/backend-secure:$CI_COMMIT_SHORT_SHA" --target=backend-secure .
podman push "$CI_REGISTRY_IMAGE/$EDITION/node-holder:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/proxy-secure:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/proxy-insecure:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/backend-auth:$CI_COMMIT_SHORT_SHA"
podman push "$CI_REGISTRY_IMAGE/$EDITION/backend-secure:$CI_COMMIT_SHORT_SHA"