#!/usr/bin/env bash

set -euo pipefail

SBOM_PATH="${RUGIX_LAYER_DIR}/artifacts/sbom.txt"
SBOM_DIR=$(dirname "${SBOM_PATH}")

if [ ! -d "${SBOM_DIR}" ]; then
    mkdir -p "${SBOM_DIR}"
fi

dpkg --list > "${SBOM_PATH}"