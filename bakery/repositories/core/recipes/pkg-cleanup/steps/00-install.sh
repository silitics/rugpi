#!/bin/sh

set -eu

if [ "${RECIPE_PARAM_AUTOREMOVE}" = "true" ]; then
    if command -v apt-get; then
        apt-get autoremove -y
    fi
fi

if command -v apt-get; then
    apt-get clean -y
    rm -rf /var/lib/apt/lists/*
fi

if command -v apk; then
    rm -rf /var/cache/apk/*
fi
