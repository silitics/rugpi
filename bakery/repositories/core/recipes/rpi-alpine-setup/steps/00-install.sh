#!/bin/sh

BOOT_DIR="${RUGPI_BUNDLE_DIR}/roots/boot"

apk add linux-rpi

cp -rp /boot/* "${BOOT_DIR}"
cp -rp "${RECIPE_DIR}/files/boot/" "${BOOT_DIR}"
