#!/usr/bin/env bash

set -euo pipefail

# Install Grub for all the different architectures.
dpkg --add-architecture amd64
dpkg --add-architecture arm64
dpkg --add-architecture armhf

apt-get update -y

apt-get install -y \
    grub-common \
    grub-efi-amd64-bin \
    grub-efi-arm-bin \
    grub-efi-arm64-bin \
    grub-efi-ia32-bin \
    grub-pc-bin \
    systemd-boot-efi:amd64 \
    systemd-boot-efi:arm64 \
    systemd-boot-efi:armhf

apt-get -y clean && rm -rf /var/lib/apt/lists/*


cd /usr/share/rugpi/boot/u-boot
./verify.sh
