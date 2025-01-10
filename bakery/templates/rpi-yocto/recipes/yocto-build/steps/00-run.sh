#!/bin/bash

set -euo pipefail

shopt -s extglob

# Place KAS directories in global build cache.
KAS_DIR="/var/rugix-bakery/cache/kas/${RUGPI_ARCH}"

mkdir -p "${KAS_DIR}"
chown build:build "${KAS_DIR}"

export KAS_WORK_DIR="${KAS_DIR}/work"
export KAS_BUILD_DIR="${KAS_DIR}/build"

# Start the actual build as the `build` user. BitBake does not like to run as `root`.
su build -c "${RECIPE_DIR}/files/build.sh"

# Save the Yocto images in the layer.
mkdir -p "${RUGPI_BUNDLE_DIR}/yocto/images/"
cp -r "${KAS_BUILD_DIR}/tmp/deploy/images/"* "${RUGPI_BUNDLE_DIR}/yocto/images/"

# Extract the root filesystem into a location where Rugpi expects it.
mkdir -p "${RUGPI_ROOT_DIR}"
ARCHIVE=$(echo "${RUGPI_BUNDLE_DIR}/yocto/images/"*"/core-image-base-"@(qemuarm64|qemux86-64)".tar.bz2")
echo "Extracting Yocto root filesystem from ${ARCHIVE}..."
tar -xjf "${ARCHIVE}" -C "${RUGPI_ROOT_DIR}"
