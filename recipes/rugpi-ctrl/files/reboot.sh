#!/bin/bash
#
# Replaces `/usr/sbin/reboot` and checks for the `force-tryboot` flag. If the flag is set,
# it forces a reboot using tryboot. This may be necessary because tools, like Mender, will
# invoke `reboot` unconditionally after an update. If the `force-tryboot` flag is not set,
# we forward any arguments to `systemctl` acting as `reboot`.

set -euo pipefail

if [ -f "/run/rugpi/flags/force-tryboot" ]; then
    exec -a /usr/sbin/reboot systemctl "0 tryboot"
else
    exec -a /usr/sbin/reboot systemctl "$@"
fi