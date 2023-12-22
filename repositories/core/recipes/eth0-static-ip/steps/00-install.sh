#!/bin/bash

set -euo pipefail

cat >/etc/network/interfaces.d/eth0 <<EOF
auto eth0
iface eth0 inet static
    address ${RECIPE_PARAM_ADDRESS}
    netmask ${RECIPE_PARAM_NETMASK}
    gateway ${RECIPE_PARAM_GATEWAY}
EOF

cat /etc/network/interfaces.d/eth0 >&2