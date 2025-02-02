#!/bin/sh

set -eu

if [ ! -x /usr/bin/rugix-ctrl ]; then
    echo "Rugix Ctrl does not exist or is not executable." >&2;
    exit 1;
fi

mkdir -p /etc/rugpi || true

if [ "${RECIPE_PARAM_RUGIX_ADMIN}" = "true" ]; then
    install -D -m 644 "${RECIPE_DIR}/files/rugix-admin.service" -t /usr/lib/systemd/system/

    systemctl enable rugix-admin
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
else
    # ln -s "$(which lsblk)" /usr/bin/lsblk
    ln -s "$(which mount)" /usr/bin/mount
    ln -s "$(which fsck)" /usr/sbin/fsck
    ln -s "$(which cp)" /usr/bin/cp
    ln -s "$(which umount)" /usr/bin/umount
    ln -s "$(which sync)" /usr/bin/sync
    # ln -s "$(which sfdisk)" /usr/sbin/sfdisk
    ln -s "$(which mkfs.ext4)" /usr/sbin/mkfs.ext4
    # ln -s "$(which findmnt)" /usr/bin/findmnt
fi

if [ ! -x /usr/bin/systemd-machine-id-setup ]; then
    cp "${RECIPE_DIR}/files/systemd-machine-id-setup" /usr/bin/systemd-machine-id-setup
    chmod 755 /usr/bin/systemd-machine-id-setup
fi

if [ ! -x /usr/sbin/sfdisk ]; then
    echo "Sfdisk is missing and cannot be installed!"
    exit 1
fi
