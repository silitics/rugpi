#!/bin/bash

set -euo pipefail

# This recipe follows the guide from the Alpine Linux wiki.
# https://wiki.alpinelinux.org/wiki/Nginx

apk add nginx

adduser -D -g 'www' www

chown -R www:www /var/lib/nginx

cat >/etc/nginx/nginx.conf <<EOF
user                www;
worker_processes    auto;

error_log   /var/log/nginx/error.log warn;
pid         /var/run/nginx/nginx.pid;

events {
    worker_connections 1024;
}

http {
    include             /etc/nginx/mime.types;
    default_type        application/octet-stream;
    sendfile            on;
    access_log          /var/log/nginx/access.log;
    keepalive_timeout   3000;

    server {
        listen          80;
        root            /var/www/html;
        index           index.html index.htm;
        server_name     localhost;
    }
}
EOF

mkdir -p /var/www
cp -rTp "${RECIPE_DIR}/html" /var/www/html
chown -R www:www /var/www/html

rc-update add nginx default
