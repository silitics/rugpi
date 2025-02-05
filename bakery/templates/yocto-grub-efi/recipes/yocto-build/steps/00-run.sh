#!/bin/bash

set -euo pipefail

shopt -s extglob

# Place KAS directories in global build cache.
KAS_DIR="/var/rugix-bakery/cache/kas/${RUGIX_ARCH}"

mkdir -p "${KAS_DIR}"
chown build:build "${KAS_DIR}"

export KAS_WORK_DIR="${KAS_DIR}/work"
export KAS_BUILD_DIR="${KAS_DIR}/build"

# Start the actual build as the `build` user. BitBake does not like to run as `root`.
su build -c "${RECIPE_DIR}/files/build.sh"

# Save the Yocto images in the layer.
mkdir -p "${RUGIX_ARTIFACTS_DIR}/yocto/images/"
cp -r "${KAS_BUILD_DIR}/tmp/deploy/images/"* "${RUGIX_ARTIFACTS_DIR}/yocto/images/"

# Extract the root filesystem into a location where Rugix expects it.
mkdir -p "${RUGIX_ROOT_DIR}"
ARCHIVE=$(echo "${RUGIX_ARTIFACTS_DIR}/yocto/images/"*"/core-image-base-"@(qemuarm64|qemux86-64)".tar.bz2")
echo "Extracting Yocto root filesystem from ${ARCHIVE}..."
tar -xjf "${ARCHIVE}" -C "${RUGIX_ROOT_DIR}"
