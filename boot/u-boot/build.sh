#!/bin/bash

set -euo pipefail

DOCKER=${DOCKER:-docker}

$DOCKER run --rm \
    -v "$(pwd)":/build/outside \
    rugpi_buildenv \
    /build/outside/mk/mk-uboot.sh
