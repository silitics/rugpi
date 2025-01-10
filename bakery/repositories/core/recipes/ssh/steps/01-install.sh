#!/bin/bash

set -euo pipefail

install -D -m 744 "${RECIPE_DIR}/files/hydrate-ssh-host-keys.sh" -t /usr/lib/rugpi/scripts/

if command -v systemctl; then
    systemctl enable ssh

    install -D -m 644 "${RECIPE_DIR}/files/systemd/hydrate-ssh-host-keys.service" -t /usr/lib/systemd/system/

    systemctl disable regenerate_ssh_host_keys || true
    systemctl enable hydrate-ssh-host-keys
fi

if command -v rc-update; then
    install -D -m 744 "${RECIPE_DIR}/files/openrc/hydrate-ssh-host-keys" -t /etc/init.d/

    rc-update add sshd
    rc-update add hydrate-ssh-host-keys
fi

if [ "${RECIPE_PARAM_ROOT_AUTHORIZED_KEYS}" != "" ]; then
    mkdir -p /root/.ssh
    echo "${RECIPE_PARAM_ROOT_AUTHORIZED_KEYS}" >> /root/.ssh/authorized_keys
    chmod -R 600 /root/.ssh
    cat /root/.ssh/authorized_keys
fi