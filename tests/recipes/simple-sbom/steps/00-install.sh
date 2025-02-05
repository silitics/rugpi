#!/usr/bin/env bash

set -euo pipefail

SBOM_PATH="${RUGIX_ARTIFACTS_DIR}/dpkg-sbom.txt"
SBOM_DIR=$(dirname "${SBOM_PATH}")

if [ ! -d "${SBOM_DIR}" ]; then
    mkdir -p "${SBOM_DIR}"
fi

dpkg --list > "${SBOM_PATH}"