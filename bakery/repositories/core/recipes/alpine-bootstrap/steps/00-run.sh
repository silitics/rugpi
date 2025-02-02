#!/bin/bash

set -euo pipefail

case ${RUGIX_ARCH} in
    "amd64")
        ALPINE_ARCH="x86_64"
        ;;
    "arm64")
        ALPINE_ARCH="aarch64"
        ;;
    "armv7")
        ALPINE_ARCH="armv7"
        ;;
    "armhf")
        ALPINE_ARCH="armhf"
        ;;
    *)
        echo "Unsupported architecture '${RUGIX_ARCH}'."
        exit 1
esac

mkdir -p "${RUGIX_ROOT_DIR}"

wget -O- "https://dl-cdn.alpinelinux.org/alpine/v${RECIPE_PARAM_VERSION}/releases/${ALPINE_ARCH}/alpine-minirootfs-${RECIPE_PARAM_VERSION}.0-${ALPINE_ARCH}.tar.gz" \
    | tar -xzvf - -C "${RUGIX_ROOT_DIR}"
