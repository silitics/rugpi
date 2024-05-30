#!/bin/sh

set -eu

if command -v apt-get; then
    apt-get update -y
fi

if command -v apk; then
    apk update
fi
