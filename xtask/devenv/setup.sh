#!/usr/bin/env bash

set -euo pipefail

HOST_ARCH=$(dpkg --print-architecture)

echo "Host Architecture: ${HOST_ARCH}"

export DEBIAN_FRONTEND=noninteractive

apt-get -y update

apt-get -y install \
    build-essential \
    curl \
    file \
    git \
    python3 \
    python3-pip \
    wget \
    zsh

wget -O /etc/zsh/zshrc https://git.grml.org/f/grml-etc-core/etc/zsh/zshrc
touch /root/.zshrc

case "${HOST_ARCH}" in
    "arm64")
        apt-get -y install gcc-x86-64-linux-gnu libc6-dev-amd64-cross
        ;;
    "amd64")
        apt-get -y install gcc-aarch64-linux-gnu libc6-dev-arm64-cross
        ;;
    *)
        echo "Error: Unsupported architecture \`${HOST_ARCH}\`."
        exit 1
        ;;
esac

# Add `armhf` toolchain regardless of host architecture.
apt-get install -y gcc-arm-linux-gnueabihf libc6-dev-armhf-cross

apt-get install -y musl-tools clang pkg-config docker.io

apt-get -y clean && rm -rf /var/lib/apt/lists/*

rustup target add arm-unknown-linux-musleabihf
rustup target add aarch64-unknown-linux-musl
rustup target add x86_64-unknown-linux-musl
