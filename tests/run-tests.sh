#!/usr/bin/env bash

set -euo pipefail

if [ $# -eq 0 ]; then
    echo "usage: $0 IMAGE_NAME"
    exit 1
fi

export RUGPI_BAKERY_IMAGE=$1

./scripts/build-images.sh

docker run --rm --privileged \
    -v "$(pwd)":/project \
    -v /dev:/dev \
    --entrypoint /bin/bash \
    "${RUGPI_BAKERY_IMAGE}" \
    /project/scripts/check-images.sh