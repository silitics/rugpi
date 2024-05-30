#!/bin/bash

set -euo pipefail

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

# XXX: Currently Rugpi expects the files for the boot partition to be directly in
# `/boot` as this was the case before Debian Bookworm. Changing this is a breaking
# change of Rugpi. We may do this with the next major release. If that happens, the
# following two lines can/must be removed.
mv /boot/firmware/* /boot
rm -rf /boot/firmware
