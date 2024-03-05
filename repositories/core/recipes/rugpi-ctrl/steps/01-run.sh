#!/bin/bash

set -euo pipefail

cp -rp "/usr/share/rugpi/binaries/${RUGPI_ARCH}/"* "${RUGPI_ROOT_DIR}/usr/bin"
