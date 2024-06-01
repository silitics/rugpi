#!/bin/bash

set -euo pipefail

function build_script() {
    local file=$1;
    local name=$2;
    ./tools/mkimage -A arm -O linux -T script -C none -n "${name}" -a 0 -e 0 \
        -d "/build/outside/scripts/${file}.uboot.sh" "/build/outside/bin/${file}.scr"
}

build_script boot "Rugpi First Stage"
build_script second "Rugpi Second Stage"
