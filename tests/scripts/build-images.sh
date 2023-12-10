#!/usr/bin/env bash

set -euo pipefail

BASE_IMAGE_URL_ARM64="https://downloads.raspberrypi.com/raspios_lite_arm64/images/raspios_lite_arm64-2023-12-06/2023-12-05-raspios-bookworm-arm64-lite.img.xz"
BASE_IMAGE_URL_ARMHF="https://downloads.raspberrypi.com/raspios_lite_armhf/images/raspios_lite_armhf-2023-12-06/2023-12-05-raspios-bookworm-armhf-lite.img.xz"

./run-bakery extract "${BASE_IMAGE_URL_ARM64}" build/base-arm64.tar
./run-bakery extract "${BASE_IMAGE_URL_ARMHF}" build/base-armhf.tar

./run-bakery customize build/base-arm64.tar build/customized-arm64.tar
./run-bakery customize build/base-armhf.tar build/customized-armhf.tar

./run-bakery --config images/tryboot-pi4-firmware.toml bake \
    build/customized-arm64.tar \
    build/tryboot-pi4-firmware.img

./run-bakery --config images/tryboot-without-firmware.toml bake \
    build/customized-arm64.tar \
    build/tryboot-without-firmware.img

./run-bakery --config images/u-boot-arm64.toml bake \
    build/customized-arm64.tar \
    build/u-boot-arm64.img

./run-bakery --config images/u-boot-armhf.toml bake \
    build/customized-armhf.tar \
    build/u-boot-armhf.img
