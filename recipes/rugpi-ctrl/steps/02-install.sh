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

install -D -m 644 "${RECIPE_DIR}/files/rugpi-admin.service" -t /usr/lib/systemd/system/

if [ "${RECIPE_PARAM_RUGPI_ADMIN}" = "true" ]; then
    systemctl enable rugpi-admin
fi