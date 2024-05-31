#!/bin/sh

set -eu

apk update

BOOT_DIR="${RUGPI_LAYER_DIR}/boot"

mkdir -p "${BOOT_DIR}"

echo "Installing kernel..."
apk add linux-lts

if [ "${RECIPE_PARAM_WITH_FIRMWARE}" = "true" ]; then
    echo "Installing firmware..."
    apk add linux-firmware
fi

echo "Copying kernel and initrd..."
cp -L /boot/vmlinuz-lts "${BOOT_DIR}/vmlinuz"
cp -L /boot/initramfs-lts "${BOOT_DIR}/initrd.img"

echo "Installing second stage boot script..."
cp "${RECIPE_DIR}/files/grub.cfg" "${BOOT_DIR}"
