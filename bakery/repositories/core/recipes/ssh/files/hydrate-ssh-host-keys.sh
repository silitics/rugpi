#!/bin/bash

set -euo pipefail

SSH_STATE_DIR=${SSH_STATE_DIR:-"/run/rugix/state/ssh"}

if [ ! -f "${SSH_STATE_DIR}"/ssh_host_rsa_key ]; then
    # Copied from `raspberrypi-sys-mods`.
    if [ -c /dev/hwrng ]; then
        dd if=/dev/hwrng of=/dev/urandom count=1 bs=4096 status=none
    fi
    rm -f /etc/ssh/ssh_host_*_key*
    ssh-keygen -A

    mkdir -p "${SSH_STATE_DIR}"
    cp /etc/ssh/ssh_host_*_key* "${SSH_STATE_DIR}"
fi

cp "${SSH_STATE_DIR}"/ssh_host_*_key* /etc/ssh/
