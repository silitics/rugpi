#!/bin/bash

set -euo pipefail

BOOT_DIR="${RUGPI_BUNDLE_DIR}/roots/boot"

RPI_UBOOT_SECOND_STAGE="/usr/share/rugpi/boot/u-boot/bin/second.scr"

cp "${RPI_UBOOT_SECOND_STAGE}" "${BOOT_DIR}"
