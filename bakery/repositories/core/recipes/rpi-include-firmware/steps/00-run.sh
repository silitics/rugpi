#!/bin/bash

set -euo pipefail

CONFIG_DIR="${RUGPI_LAYER_DIR}/config"

RPI_EEPROM_DIGEST="/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest"

PI4_FIRMWARE="/usr/share/rugpi/rpi-eeprom/firmware-2711"
PI5_FIRMWARE="/usr/share/rugpi/rpi-eeprom/firmware-2712"

mkdir -p "${CONFIG_DIR}"

case "${RECIPE_PARAM_MODEL}" in
    "pi4")
        cp -f "${PI4_FIRMWARE}/stable/pieeprom-2023-05-11.bin" "${CONFIG_DIR}/pieeprom.upd"
        ${RPI_EEPROM_DIGEST} -i "${CONFIG_DIR}/pieeprom.upd" -o "${CONFIG_DIR}/pieeprom.sig"
        cp -f "${PI4_FIRMWARE}/stable/vl805-000138c0.bin" "${CONFIG_DIR}/vl805.bin"
        ${RPI_EEPROM_DIGEST} -i "${CONFIG_DIR}/vl805.bin" -o "${CONFIG_DIR}/vl805.sig"
        cp -f "${PI4_FIRMWARE}/stable/recovery.bin" "${CONFIG_DIR}/recovery.bin"
        ;;
    "pi5")
        cp -f "${PI5_FIRMWARE}/stable/pieeprom-2023-10-30.bin" "${CONFIG_DIR}/pieeprom.upd"
        ${RPI_EEPROM_DIGEST} -i "${CONFIG_DIR}/pieeprom.upd" -o "${CONFIG_DIR}/pieeprom.sig"
        cp -f "${PI5_FIRMWARE}/stable/recovery.bin" "${CONFIG_DIR}/recovery.bin"
        ;;
    *)
        echo "Error: Invalid Raspberry Pi model '${RECIPE_PARAM_MODEL}'."
        exit 1
        ;;
esac
