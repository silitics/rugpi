#!/bin/bash

set -euo pipefail

dpkg --add-architecture amd64

apt-get update -y
apt-get install -y systemd-boot:amd64

mkdir -p "${RUGPI_ROOT_DIR}/boot/efi/EFI/BOOT"
cp /usr/lib/systemd/boot/efi/systemd-bootx64.efi "${RUGPI_ROOT_DIR}/boot/efi/EFI/BOOT/BOOTX64.EFI"

mkdir -p "${RUGPI_ROOT_DIR}/boot/efi/loader/entries"
cat >"${RUGPI_ROOT_DIR}/boot/efi/loader/loader.conf" <<EOF
default rugpi-a.conf
timeout 10
console-mode max
editor yes
EOF

cat >"${RUGPI_ROOT_DIR}/boot/efi/loader/entries/rugpi-a.conf" <<EOF
title Rugpi System A
linux /rugpi/a/vmlinuz
initrd /rugpi/a/initrd.img
options root=/dev/sda2 rw
EOF

mkdir -p "${RUGPI_ROOT_DIR}/boot/efi/rugpi/a"
cp "${RUGPI_ROOT_DIR}/vmlinuz" "${RUGPI_ROOT_DIR}/boot/efi/rugpi/a"
cp "${RUGPI_ROOT_DIR}/initrd.img" "${RUGPI_ROOT_DIR}/boot/efi/rugpi/a"
