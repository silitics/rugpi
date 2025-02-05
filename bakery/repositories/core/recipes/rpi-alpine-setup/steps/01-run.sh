#!/bin/sh

BOOT_DIR="${RUGIX_LAYER_DIR}/roots/boot"

# Install firmware files for Raspberry Pi 4.
wget -O "${BOOT_DIR}/start4.elf" \
    https://github.com/raspberrypi/firmware/raw/master/boot/start4.elf
wget -O "${BOOT_DIR}/fixup4.dat" \
    https://github.com/raspberrypi/firmware/raw/master/boot/fixup4.dat