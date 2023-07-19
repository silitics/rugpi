#!/bin/bash

wget -O "${RUGPI_ROOT_DIR}/etc/zsh/zshrc" https://git.grml.org/f/grml-etc-core/etc/zsh/zshrc
touch "${RUGPI_ROOT_DIR}/root/.zshrc"
