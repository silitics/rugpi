#!/bin/bash

set -euo pipefail

systemctl enable systemd-networkd.service

install -m 644 "${RECIPE_DIR}/files/dhcp.network" "/etc/systemd/network/"
