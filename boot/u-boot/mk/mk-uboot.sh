#!/bin/bash

set -euo pipefail

export SOURCE_DATE_EPOCH="1700611200"

UBOOT_VERSION="2023.10"
UBOOT_HASH="669994eab941bdfb00981bd174713fe4ae414bba6190707ecba8b23d74dde037"

UBOOT_SRC_ZIP="/build/outside/.cache/v${UBOOT_VERSION}.zip"

mkdir -p /build/outside/.cache

if [ ! -f "${UBOOT_SRC_ZIP}" ]; then
    echo "Downloading U-Boot sources (version: ${UBOOT_VERSION})..."
    wget -O "${UBOOT_SRC_ZIP}" https://github.com/u-boot/u-boot/archive/refs/tags/v2023.10.zip
fi

echo "Checking U-Boot sources for integrity..."
echo "${UBOOT_HASH} ${UBOOT_SRC_ZIP}" >"/tmp/uboot-zip.sha256"
sha256sum --strict -c /tmp/uboot-zip.sha256

echo "Extracting sources..."
unzip "${UBOOT_SRC_ZIP}" >/dev/null
mv u-boot-${UBOOT_VERSION} u-boot

cd u-boot
cp /build/outside/configs/* configs/


function build_uboot() {
    local name=$1;
    local arch=$2;
    local config=$3;

    make clean

    case ${arch} in
        "armhf")
            export CROSS_COMPILE=/usr/bin/arm-linux-gnueabi-
            ;;
        "arm64")
            export CROSS_COMPILE=/usr/bin/aarch64-linux-gnu-
            ;;
    esac

    make "${config}"
    make -j "$(nproc)"

    mv u-boot.bin "/build/outside/bin/u-boot-${name}.bin"
}

mkdir -p /build/outside/bin

build_uboot arm64 arm64 rpi_arm64_rugpi_defconfig

build_uboot armhf-zerow armhf rpi_armhf_zerow_rugpi_defconfig
build_uboot armhf-pi1 armhf rpi_armhf_pi1_rugpi_defconfig
build_uboot armhf-pi2 armhf rpi_armhf_pi2_rugpi_defconfig
build_uboot armhf-pi3 armhf rpi_armhf_pi3_rugpi_defconfig

/build/outside/mk/mk-scripts.sh

if [ -f "/build/outside/checksums.txt" ]; then
    sha256sum --strict -c "/build/outside/checksums.txt"
else
    sha256sum /build/outside/bin/* >/build/outside/checksums.txt
fi
