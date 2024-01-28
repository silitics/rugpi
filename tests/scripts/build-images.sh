#!/usr/bin/env bash

set -euo pipefail

./run-bakery bake image pi4 build/tryboot-pi4-firmware.img

./run-bakery bake image tryboot build/tryboot-without-firmware.img

./run-bakery bake image u-boot build/u-boot-arm64.img

./run-bakery bake image u-boot-armhf build/u-boot-armhf.img
