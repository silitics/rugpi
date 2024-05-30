#!/bin/bash

set -euo pipefail

case "${RUGPI_ARCH}" in
    "amd64")
        DEBIAN_ARCH="amd64"
        ;;
    "arm64")
        DEBIAN_ARCH="arm64"
        ;;
    "armv7")
        DEBIAN_ARCH="armhf"
        ;;
    "arm")
        DEBIAN_ARCH="armel"
        ;;
    *)
        echo "Unsupported architecture '${RUGPI_ARCH}'."
        exit 1
esac

mmdebstrap \
    --architectures="${DEBIAN_ARCH}" \
    "${RECIPE_PARAM_SUITE}" \
    "${RUGPI_ROOT_DIR}"
