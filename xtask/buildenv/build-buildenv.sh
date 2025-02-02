#!/bin/bash

set -euo pipefail

DOCKER=${DOCKER:-docker}

source lock.env

$DOCKER build \
    --build-arg "RUST_VERSION=${RUST_VERSION}" \
    -t rugix_buildenv -f Dockerfile.buildenv .
