#!/bin/bash

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get purge -y dphys-swapfile
