#!/bin/bash

set -euo pipefail

mkdir -p /etc/rugix
cp "${RECIPE_DIR}/files/system.toml" /etc/rugix/system.toml
