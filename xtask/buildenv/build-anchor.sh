#!/bin/bash

set -euo pipefail

DOCKER=${DOCKER:-docker}

source lock.env

$DOCKER build \
    --build-arg "SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}" \
    --build-arg "DEBIAN_SNAPSHOT=${DEBIAN_SNAPSHOT}" \
    --build-arg "DEBIAN_SUITE=${DEBIAN_SUITE}" \
    -t rugpi_buildenv_anchor_stage0 -f Dockerfile.stage0 .
    
$DOCKER run --rm --privileged \
    -v "$(pwd)":/build/outside \
    rugpi_buildenv_anchor_stage0 \
    /build/bootstrap.sh

$DOCKER build -t rugpi_buildenv_anchor -f Dockerfile.stage1 .
