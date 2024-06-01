#!/bin/bash

set -euo pipefail

systemctl enable nginx

rm -rf /var/www/html
cp -rTp "${RECIPE_DIR}/html" /var/www/html
chown -R www-data:www-data /var/www/html
