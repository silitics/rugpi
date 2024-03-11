#!/bin/bash

set -euo pipefail

systemctl enable ssh

install -D -m 744 "${RECIPE_DIR}/files/hydrate-ssh-host-keys.sh" -t /usr/lib/rugpi/scripts/
install -D -m 644 "${RECIPE_DIR}/files/hydrate-ssh-host-keys.service" -t /usr/lib/systemd/system/

systemctl disable regenerate_ssh_host_keys || true
systemctl enable hydrate-ssh-host-keys

if [ "${RECIPE_PARAM_ROOT_AUTHORIZED_KEYS}" != "" ]; then
    mkdir -p /root/.ssh
    echo "${RECIPE_PARAM_ROOT_AUTHORIZED_KEYS}" >> /root/.ssh/authorized_keys
    chmod -R 600 /root/.ssh
    cat /root/.ssh/authorized_keys >&2
fi