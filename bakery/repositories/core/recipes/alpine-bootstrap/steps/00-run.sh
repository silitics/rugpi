#!/bin/bash

set -euo pipefail

case ${RUGPI_ARCH} in
    "amd64")
        ALPINE_ARCH="x86_64"
        ;;
    "arm64")
        ALPINE_ARCH="aarch64"
        ;;
    "armhf")
        ALPINE_ARCH="armhf"
        ;;
    *)
        echo "Unsupported architecture ${RUGPI_ARCH}."
        exit 1
esac

wget -O /tmp/alpine-rootfs.tar.gz \
    "https://dl-cdn.alpinelinux.org/alpine/v${RECIPE_PARAM_VERSION}/releases/${ALPINE_ARCH}/alpine-minirootfs-${RECIPE_PARAM_VERSION}.0-${ALPINE_ARCH}.tar.gz"


mkdir -p "${RUGPI_ROOT_DIR}"
tar -xvf /tmp/alpine-rootfs.tar.gz -C "${RUGPI_ROOT_DIR}"
