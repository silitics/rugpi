#!/usr/bin/env bash

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get -y update

apt-get -y install pkg-config

apt-get -y clean && rm -rf /var/lib/apt/lists/*
