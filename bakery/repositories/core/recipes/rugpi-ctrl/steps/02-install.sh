#!/bin/sh

set -eu

if [ ! -x /usr/bin/rugpi-ctrl ]; then
    echo "Rugpi Ctrl does not exist or is not executable." >&2;
    exit 1;
fi

mkdir -p /etc/rugpi || true

if [ "${RECIPE_PARAM_RUGPI_ADMIN}" = "true" ]; then
    install -D -m 644 "${RECIPE_DIR}/files/rugpi-admin.service" -t /usr/lib/systemd/system/

    systemctl enable rugpi-admin
fi


prog_exists() {
    prog="$1";
    command -v "$prog" >/dev/null 2>&1
}

if prog_exists apt; then
    apt-get install -y fdisk
elif prog_exists apk; then
    mkdir -p /etc/rugpi || true

    apk add sfdisk e2fsprogs lsblk findmnt e2fsprogs-extra dosfstools

    ln -s "$(which lsblk)" /usr/bin/lsblk
    ln -s "$(which mount)" /usr/bin/mount
    ln -s "$(which fsck)" /usr/sbin/fsck
    ln -s "$(which cp)" /usr/bin/cp
    ln -s "$(which umount)" /usr/bin/umount
    ln -s "$(which sync)" /usr/bin/sync
    ln -s "$(which sfdisk)" /usr/sbin/sfdisk
    ln -s "$(which mkfs.ext4)" /usr/sbin/mkfs.ext4
    ln -s "$(which findmnt)" /usr/bin/findmnt

    install -D -m 755 "${RECIPE_DIR}/files/systemd-machine-id-setup" -t /usr/bin

    setup-interfaces -a
fi

if [ ! -x /usr/sbin/sfdisk ]; then
    echo "Sfdisk is missing and cannot be installed!"
    exit 1
fi
