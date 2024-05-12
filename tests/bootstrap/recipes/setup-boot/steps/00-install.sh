#!/bin/bash

set -euo pipefail

cat >/etc/apt/sources.list <<EOF
deb http://deb.debian.org/debian bookworm main contrib non-free non-free-firmware
deb http://deb.debian.org/debian bookworm-updates main contrib non-free non-free-firmware
deb http://security.debian.org/debian-security bookworm-security main contrib non-free non-free-firmware
EOF

apt-get update -y

install -D -m 644 "${RECIPE_DIR}/files/ctrl.toml" -t /etc/rugpi
