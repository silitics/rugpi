#!/bin/bash

set -euo pipefail

MMDEBSTRAP_VERSION=$(mmdebstrap --version)
DPKG_ARCHITECTURE=$(dpkg --print-architecture)

BUILD_INFO="
SOURCE_DATE_EPOCH='${SOURCE_DATE_EPOCH}'
DEBIAN_SNAPSHOT='${DEBIAN_SNAPSHOT}'
DEBIAN_SUITE='${DEBIAN_SUITE}'
MMDEBSTRAP_VERSION='${MMDEBSTRAP_VERSION}'
DPKG_ARCHITECTURE='${DPKG_ARCHITECTURE}'
"

echo "$BUILD_INFO"

mmdebstrap \
    --format=tar \
    --aptopt='Acquire::Check-Valid-Until "false"' \
    --aptopt='Apt::Key::gpgvcommand "/usr/libexec/mmdebstrap/gpgvnoexpkeysig"' \
    --customize-hook='/build/customize.sh' \
    --include="ca-certificates mmdebstrap" \
    "${DEBIAN_SUITE}" \
    /build/outside/rootfs.tar \
    "https://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/"
