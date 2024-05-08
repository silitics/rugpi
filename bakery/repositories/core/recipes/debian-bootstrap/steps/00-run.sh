#!/bin/bash

set -euo pipefail

mmdebstrap \
    --architectures="${RUGPI_ARCH}" \
    "${RECIPE_PARAM_SUITE}" \
    "${RUGPI_ROOT_DIR}"
