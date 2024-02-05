#!/usr/bin/env bash

set -euo pipefail

TEMP_DIR=$(mktemp -d)

CONFIG_DIR="${TEMP_DIR}/config"
BOOT_DIR="${TEMP_DIR}/boot"

LOOP_DEV=$(losetup -f)

CONFIG_PARTITION="${LOOP_DEV}p1"
BOOT_PARTITION="${LOOP_DEV}p2"

function cleanup() {
    umount "${CONFIG_DIR}" 2>/dev/null || true
    umount "${BOOT_DIR}" 2>/dev/null || true
    losetup -d "${LOOP_DEV}" 2>/dev/null || true
    rm -rf "${TEMP_DIR}"
}

trap cleanup EXIT

mkdir -p "${CONFIG_DIR}"
mkdir -p "${BOOT_DIR}"

function mount_image() {
    local image=$1
    losetup -P "${LOOP_DEV}" "${image}"
    mount "${CONFIG_PARTITION}" "${CONFIG_DIR}"
    mount "${BOOT_PARTITION}" "${BOOT_DIR}"
}

function umount_image() {
    umount "${CONFIG_DIR}"
    umount "${BOOT_DIR}"
    losetup -d "${LOOP_DEV}"
}


function check_boot() {
    ls -l "${BOOT_DIR}"
    if [ -f "${BOOT_DIR}/second.scr" ] \
        && [ -f "${BOOT_DIR}/boot.env" ] \
        && [ -f "${BOOT_DIR}/config.txt" ]
    then
        echo "Boot. Ok."
    else
        echo "Boot. Error."
        return 1
    fi
}

function check_tryboot() {
    ls -l "${CONFIG_DIR}"
    if [ -f "${CONFIG_DIR}/autoboot.txt" ] \
        && [ -f "${CONFIG_DIR}/autoboot.a.txt" ] \
        && [ -f "${CONFIG_DIR}/autoboot.b.txt" ]
    then
        echo "Tryboot. Ok."
    else
        echo "Tryboot. Error."
        return 1
    fi
}

function check_firmware() {
    if [ -f "${CONFIG_DIR}/recovery.bin" ] \
        && [ -f "${CONFIG_DIR}/pieeprom.upd" ] \
        && [ -f "${CONFIG_DIR}/pieeprom.sig" ]
    then
        echo "Firmware. Ok."
    else
        echo "Firmware. Error."
        return 1
    fi
}

function check_not_firmware() {
    if [ -f "${CONFIG_DIR}/recovery.bin" ] \
        || [ -f "${CONFIG_DIR}/pieeprom.upd" ] \
        || [ -f "${CONFIG_DIR}/pieeprom.sig" ]
    then
        echo "Firmware found. Error."
        return 1
    else
        echo "No firmware. Ok."
    fi
}

function check_u_boot() {
    if [ -f "${CONFIG_DIR}/config.txt" ] \
        && [ -f "${CONFIG_DIR}/boot.scr" ] \
        && [ -f "${CONFIG_DIR}/bootpart.default.env" ] \
        && [ -f "${CONFIG_DIR}/boot_spare.disabled.env" ] \
        && [ -f "${CONFIG_DIR}/boot_spare.enabled.env" ]
    then
        echo "U-Boot. Ok."
    else
        echo "U-Boot. Error"
        return 1
    fi
}


mount_image "build/tryboot-pi4-firmware.img"
echo "Checking 'tryboot-pi4-firmware.img'"
check_tryboot
check_firmware
check_boot
umount_image

mount_image "build/tryboot-without-firmware.img"
echo "Checking 'tryboot-without-firmware.img'"
check_tryboot
check_not_firmware
[ ! -f "${CONFIG_DIR}/pieeprom.upd" ]
umount_image

mount_image "build/u-boot-arm64.img"
echo "Checking 'u-boot-arm64.img'"
ls -l "${CONFIG_DIR}"
[ -f "${CONFIG_DIR}/u-boot-arm64.bin" ]
check_u_boot
check_boot
check_not_firmware
umount_image

mount_image "build/u-boot-armhf.img"
echo "Checking 'u-boot-arhf.img'"
ls -l "${CONFIG_DIR}"
[ -f "${CONFIG_DIR}/u-boot-armhf-pi1.bin" ]
[ -f "${CONFIG_DIR}/u-boot-armhf-pi2.bin" ]
[ -f "${CONFIG_DIR}/u-boot-armhf-pi3.bin" ]
[ -f "${CONFIG_DIR}/u-boot-armhf-zerow.bin" ]
check_u_boot
check_boot
check_not_firmware
umount_image