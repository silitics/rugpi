#!/bin/sh

# Install firmware files for Raspberry Pi 4.
wget -O "${RUGPI_ROOT_DIR}"/boot/start4.elf \
    https://github.com/raspberrypi/firmware/raw/master/boot/start4.elf
wget -O "${RUGPI_ROOT_DIR}"/boot/fixup4.dat \
    https://github.com/raspberrypi/firmware/raw/master/boot/fixup4.dat