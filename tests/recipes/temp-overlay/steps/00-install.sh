#!/usr/bin/env bash

mkdir -p /etc/rugix
cat >/etc/rugix/state.toml <<EOF
overlay = "in-memory"
EOF