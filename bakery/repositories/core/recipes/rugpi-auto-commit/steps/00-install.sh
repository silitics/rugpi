#!/bin/bash

set -euo pipefail

install -D -m 644 "${RECIPE_DIR}/files/rugix-auto-commit.service" -t /usr/lib/systemd/system/

systemctl enable rugix-auto-commit
