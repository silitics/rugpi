#!/bin/bash

set -euo pipefail

shopt -s extglob

BOOT_DIR="${RUGIX_LAYER_DIR}/roots/boot"

mkdir -p "${BOOT_DIR}"

echo "Copying kernel..."
cp -L "${RUGIX_ROOT_DIR}/boot/"@(bzImage|Image) "${BOOT_DIR}/vmlinuz"

echo "Installing second stage boot script..."
cp "${RECIPE_DIR}/files/grub.cfg" "${BOOT_DIR}"
