#!/bin/bash

set -euo pipefail

if [ ! -x /usr/bin/rugpi-ctrl ]; then
    echo "Rugpi Ctrl does not exist or is not executable." >&2;
    exit 1;
fi

ldd /usr/bin/rugpi-ctrl >&2 || true

mkdir -p /etc/rugpi || true

cat >/etc/rugpi/ctrl.toml <<EOF
system_size = "${RECIPE_PARAM_SYSTEM_SIZE}"
EOF
