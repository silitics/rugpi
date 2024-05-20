#!/usr/bin/env bash

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get -y update

apt-get -y install \
    btrfs-progs \
    curl \
    dosfstools \
    fdisk \
    file \
    git \
    gpg \
    mmdebstrap \
    mtools \
    proot \
    python3 \
    qemu-utils \
    wget \
    xz-utils \
    zip \
    zsh

apt-get -y clean && rm -rf /var/lib/apt/lists/*

wget -O /etc/zsh/zshrc https://git.grml.org/f/grml-etc-core/etc/zsh/zshrc
touch /root/.zshrc
