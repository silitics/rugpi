#!/bin/bash

set -euo pipefail

DOCKER=${DOCKER:-docker}
IMAGE_NAME=${IMAGE_NAME:-rugpi-uboot}

${DOCKER} build -t "${IMAGE_NAME}" .

# Create a container such that we can extract the binary files.
rm -rf bin
container=$(${DOCKER} create "${IMAGE_NAME}")
${DOCKER} cp "${container}:/home/uboot/u-boot-2023.10/out" "bin"
${DOCKER} rm "${container}"

# Check or create checksums.
cd bin
if [ -e ../checksums.txt ]; then
    sha256sum --strict -c ../checksums.txt
else
    sha256sum ./* > ../checksums.txt
    cat ../checksums.txt
fi
