#!/bin/bash

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

if [ "${RECIPE_PARAM_AUTOREMOVE}" = "true" ]; then
    apt-get autoremove -y
fi

apt-get clean -y
rm -rf /var/lib/apt/lists/*
