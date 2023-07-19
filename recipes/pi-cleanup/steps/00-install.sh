#!/bin/bash

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

systemctl disable userconfig
systemctl disable unattended-upgrades.service
systemctl disable resize2fs_once

apt-get purge -y userconf-pi
rm -f /etc/ssh/sshd_config.d/rename_user.conf
