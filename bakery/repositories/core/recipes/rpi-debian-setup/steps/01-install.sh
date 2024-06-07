#!/bin/bash

set -euo pipefail

BOOT_DIR="${RUGPI_BUNDLE_DIR}/roots/boot/"

install -m 644 "${RECIPE_DIR}/files/raspberrypi.list" "/etc/apt/sources.list.d/"
sed -i "s/RELEASE/bookworm/g" "/etc/apt/sources.list.d/raspberrypi.list"

apt-get update -y
apt-get install -y raspberrypi-archive-keyring

apt-get install -y \
    initramfs-tools \
    raspi-firmware \
    linux-image-rpi-v8 \
    linux-headers-rpi-v8 \
    linux-image-rpi-2712 \
    linux-headers-rpi-2712

install -m 644 "${RECIPE_DIR}/files/cmdline.txt" "/boot/firmware/"
install -m 644 "${RECIPE_DIR}/files/config.txt" "/boot/firmware/"

mkdir -p "${BOOT_DIR}"
cp -rp /boot/firmware/* "${BOOT_DIR}"
