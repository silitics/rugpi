#!/bin/bash

set -euo pipefail

case "${RUGPI_ARCH}" in
    "amd64")
        TARGET="x86_64-unknown-linux-musl"
        ;;
    "arm64")
        TARGET="aarch64-unknown-linux-musl"
        ;;
    "armv7")
        TARGET="armv7-unknown-linux-musleabihf"
        ;;
    "armhf")
        TARGET="arm-unknown-linux-musleabihf"
        ;;
    "arm")
        TARGET="arm-unknown-linux-musleabi"
        ;;
    *)
        echo "Unsupported architecture '${RUGPI_ARCH}'."
        exit 1
esac

cp "/usr/share/rugpi/binaries/${TARGET}/rugpi-ctrl" "${RUGPI_ROOT_DIR}/usr/bin"

if [ "${RECIPE_PARAM_RUGPI_ADMIN}" = "true" ]; then
    cp "/usr/share/rugpi/binaries/${TARGET}/rugpi-admin" "${RUGPI_ROOT_DIR}/usr/bin"
fi
