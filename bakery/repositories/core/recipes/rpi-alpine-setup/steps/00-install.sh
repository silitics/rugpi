#!/bin/sh

BOOT_DIR="${RUGIX_LAYER_DIR}/roots/boot/"

apk add linux-rpi

mkdir -p "${BOOT_DIR}"

cp -rp /boot/* "${BOOT_DIR}"
cp -rp "${RECIPE_DIR}/files/boot/"* "${BOOT_DIR}"
