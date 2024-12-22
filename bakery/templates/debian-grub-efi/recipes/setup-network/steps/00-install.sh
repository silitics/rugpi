#!/bin/bash

set -euo pipefail

systemctl enable systemd-networkd.service

install -m 644 "${RECIPE_DIR}/files/dhcp.network" "/etc/systemd/network/"

# Setup `systemd-resolved`.
mkdir -p /run/systemd/resolve
cat /etc/resolv.conf > /run/systemd/resolve/stub-resolv.conf
apt-get install -y systemd-resolved
systemctl enable systemd-resolved.service
rm -f /etc/resolv.conf
ln -s /run/systemd/resolve/stub-resolv.conf /etc/resolv.conf 
