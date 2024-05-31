#!/bin/bash

set -euo pipefail

export SOURCE_DATE_EPOCH="1717152985"

GRUB_VERSION="2.12"
GRUB_HASH="b30919fa5be280417c17ac561bb1650f60cfb80cc6237fa1e2b6f56154cb9c91"

GRUB_SRC_TAR="/build/outside/.cache/grub-${GRUB_VERSION}.tar.gz"

mkdir -p /build/outside/.cache

if [ ! -f "${GRUB_SRC_TAR}" ]; then
    echo "Downloading Grub sources (version: ${GRUB_VERSION})..."
    wget -O "${GRUB_SRC_TAR}" ftp://ftp.gnu.org/gnu/grub/grub-${GRUB_VERSION}.tar.gz
fi

echo "Checking Grub sources for integrity..."
echo "${GRUB_HASH} ${GRUB_SRC_TAR}" >"/tmp/grub-tar.sha256"
sha256sum --strict -c /tmp/grub-tar.sha256

echo "Extracting sources..."
tar -zxvf "${GRUB_SRC_TAR}"
cp -rp grub-${GRUB_VERSION} grub


function build_grub() {
    local arch=$1;
    local host=$2;

    cp -rp grub "grub-${arch}"
    cd "grub-${arch}"

    touch grub-core/extra_deps.lst
    ./configure \
        --prefix "/opt/grub-${arch}" \
        --with-platform=efi \
        --disable-efiemu \
        --host "${host}"
    
    make -j "$(nproc)"
    make install

    cd ..
}

build_grub arm64 aarch64-linux-gnu
build_grub amd64 x86_64-linux-gnu

/build/outside/mk/mk-efi-images.sh

if [ -f "/build/outside/checksums.txt" ]; then
    sha256sum --strict -c "/build/outside/checksums.txt"
else
    sha256sum /build/outside/bin/* >/build/outside/checksums.txt
fi
