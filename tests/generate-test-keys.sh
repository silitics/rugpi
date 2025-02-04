#!/usr/bin/env bash

set -euo pipefail

rm -rf keys
mkdir keys
ssh-keygen -t rsa -b 4096 -f keys/id_rsa -q -N ""