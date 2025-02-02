#!/bin/bash

set -euo pipefail

case "${RUGIX_ARCH}" in
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
        echo "Unsupported architecture '${RUGIX_ARCH}'."
        exit 1
esac

cp "/usr/share/rugix/binaries/${TARGET}/rugix-ctrl" "${RUGIX_ROOT_DIR}/usr/bin"

if [ "${RECIPE_PARAM_RUGIX_ADMIN}" = "true" ]; then
    cp "/usr/share/rugix/binaries/${TARGET}/rugix-admin" "${RUGIX_ROOT_DIR}/usr/bin"
fi
