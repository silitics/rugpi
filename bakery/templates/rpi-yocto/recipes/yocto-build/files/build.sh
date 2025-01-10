#!/bin/bash

set -euo pipefail


# Setup Python virtual environment to install a recent version of KAS.

VENV_DIR=$(mktemp -d)

function cleanup() {
    rm -rf "${VENV_DIR}"
}

trap cleanup EXIT

python3 -m venv "${VENV_DIR}"
source "${VENV_DIR}/bin/activate"

python3 -m pip install kas


# Prepare environment and run KAS build.

mkdir -p "${KAS_WORK_DIR}"
mkdir -p "${KAS_BUILD_DIR}"

kas build "${RECIPE_DIR}/files/kas-config-rpi4.yaml"
