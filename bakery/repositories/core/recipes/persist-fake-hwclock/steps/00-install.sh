#!/bin/bash

set -euo pipefail

install -D -m 644 "${RECIPE_DIR}/files/fake-hwclock.toml" -t /etc/rugix/state
