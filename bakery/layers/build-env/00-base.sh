#!/usr/bin/env bash

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get -y update

apt-get -y install \
    build-essential \
    curl \
    wget \
    zsh \
    git \
    file \
    python3 \
    python3-pip

apt-get -y clean && rm -rf /var/lib/apt/lists/*

wget -O /etc/zsh/zshrc https://git.grml.org/f/grml-etc-core/etc/zsh/zshrc
touch /root/.zshrc
