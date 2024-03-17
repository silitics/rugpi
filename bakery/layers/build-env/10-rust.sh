#!/usr/bin/env bash

set -euo pipefail

DEBIAN_ARCH=$(dpkg --print-architecture)

echo "Architecture: ${DEBIAN_ARCH}"

export DEBIAN_FRONTEND=noninteractive


# Add `armhf` toolchain regardless of host architecture.
dpkg --add-architecture armhf
apt-get -y update
apt-get install -y gcc-arm-linux-gnueabihf libc6:armhf


# Add `arm64` toolchain on `amd64` only.
case "${DEBIAN_ARCH}" in
    "arm64")
        ;;
    "amd64")
        dpkg --add-architecture arm64
        apt-get -y update
        apt-get -y install gcc-aarch64-linux-gnu libc6:arm64
        ;;
    *)
        echo "Error: Unsupported architecture \`${DEBIAN_ARCH}\`."
        exit 1
        ;;
esac


apt-get install -y clang

apt-get -y clean && rm -rf /var/lib/apt/lists/*


curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --default-toolchain "${RUST_VERSION}"

rustup target add arm-unknown-linux-musleabihf  # Raspberry Pi (32-bit)
rustup target add aarch64-unknown-linux-musl  # Raspberry Pi (64-bit)

cargo install cargo-chef --locked
