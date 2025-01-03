#!/bin/bash

set -euo pipefail

install -m 755 "${RECIPE_DIR}/files/rugpi-system-assert" "/usr/bin/"
