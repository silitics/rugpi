#!/bin/sh

apk add linux-rpi

cp -rp "${RECIPE_DIR}/files/boot/"* /boot

# Remove spurious symlink.
rm -f /boot/boot