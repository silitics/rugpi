#!/bin/bash

set -euo pipefail

BOOT_DIR="${RUGIX_LAYER_DIR}/roots/boot"

RPI_UBOOT_SECOND_STAGE="/usr/share/rugix/boot/u-boot/bin/second.scr"

cp "${RPI_UBOOT_SECOND_STAGE}" "${BOOT_DIR}"
