#!/bin/bash

set -euo pipefail

echo "Verifying integrity of U-Boot binaries..."

cd bin
sha256sum --strict -c ../checksums.txt
