#!/bin/bash

set -euo pipefail

case "${RUGPI_ARCH}" in
    "armhf")
        TARGET="arm-unknown-linux-musleabihf"
        ;;
    "arm64")
        TARGET="aarch64-unknown-linux-musl"
        ;;
esac

cp "/usr/share/rugpi/binaries/${TARGET}/rugpi-ctrl" "${RUGPI_ROOT_DIR}/usr/bin"

if [ "${RECIPE_PARAM_RUGPI_ADMIN}" = "true" ]; then
    cp "/usr/share/rugpi/binaries/${TARGET}/rugpi-admin" "${RUGPI_ROOT_DIR}/usr/bin"
fi
