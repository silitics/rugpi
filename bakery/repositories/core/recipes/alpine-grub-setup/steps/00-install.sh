#!/bin/sh

set -eu

apk update

BOOT_DIR="${RUGIX_LAYER_DIR}/roots/boot"

mkdir -p "${BOOT_DIR}"

echo "Installing and configuring 'mkinitfs'..."
apk add mkinitfs util-linux

cp "${RECIPE_DIR}/files/mkinitfs/features.d/rugix.files" "/etc/mkinitfs/features.d/rugix.files"
cp "${RECIPE_DIR}/files/mkinitfs/mkinitfs.conf" "/etc/mkinitfs/mkinitfs.conf"

echo "Installing kernel..."
apk add linux-lts

echo "Copying kernel and initrd..."
cp -L /boot/vmlinuz-lts "${BOOT_DIR}/vmlinuz"
cp -L /boot/initramfs-lts "${BOOT_DIR}/initrd.img"

echo "Installing second stage boot script..."
cp "${RECIPE_DIR}/files/grub.cfg" "${BOOT_DIR}"
