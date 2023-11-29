#!/bin/bash

set -euo pipefail

mkdir -p out

function build_uboot() {
    local name=$1;
    local arch=$2;
    local config=$3;

    make clean

    case ${arch} in
        32)
            export CROSS_COMPILE=/opt/gcc-13.2.0-nolibc/arm-linux-gnueabi/bin/arm-linux-gnueabi-
            ;;
        64)
            export CROSS_COMPILE=/opt/gcc-13.2.0-nolibc/aarch64-linux/bin/aarch64-linux-
            ;;
    esac

    make ${config}
    make -j$(nproc)

    mv u-boot.bin out/u-boot-${name}.bin
}

build_uboot arm64 64 rpi_arm64_rugpi_defconfig
build_uboot armhf-zerow 32 rpi_armhf_zerow_rugpi_defconfig
build_uboot armhf-pi1 32 rpi_armhf_pi1_rugpi_defconfig
build_uboot armhf-pi2 32 rpi_armhf_pi2_rugpi_defconfig