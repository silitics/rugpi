#!/bin/bash

set -euo pipefail

echo "669994eab941bdfb00981bd174713fe4ae414bba6190707ecba8b23d74dde037 v2023.10.zip" > v2023.10.zip.sha256

wget https://github.com/u-boot/u-boot/archive/refs/tags/v2023.10.zip

sha256sum --strict -c v2023.10.zip.sha256

unzip v2023.10.zip
