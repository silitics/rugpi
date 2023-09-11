#!/usr/bin/env bash

set -euo pipefail

# Enable discovery of the SSH service.
cp /usr/share/doc/avahi-daemon/examples/ssh.service /etc/avahi/services

# Enable discovery of the SFTP service.
cp /usr/share/doc/avahi-daemon/examples/sftp-ssh.service /etc/avahi/services

# Enable discovery of the HTTP interface.
install -D -m 644 "${RECIPE_DIR}/files/http.service" -t /etc/avahi/services
