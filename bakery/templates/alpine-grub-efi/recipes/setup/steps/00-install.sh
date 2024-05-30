#!/bin/sh

set -eu

apk add linux-lts

ln -s /boot/vmlinuz-lts /vmlinuz
ln -s /boot/initramfs-lts /initrd.img

apk add linux-firmware
