#!/bin/bash

set -euo pipefail

MEMDISK_SIZE=64

# This has been taken from Debian with the addition of `hashsum`. We require `hashsum`
# as we are using SHA1 hashes to verify that environment files are good.
GRUB_MODULES="
    all_video
    boot
    btrfs
    cat
    chain
    configfile
    cryptodisk
    echo
    efifwsetup
    efinet
    ext2
    f2fs
    fat
    font
    gcry_arcfour
    gcry_blowfish
    gcry_camellia
    gcry_cast5
    gcry_crc
    gcry_des
    gcry_dsa
    gcry_idea
    gcry_md4
    gcry_md5
    gcry_rfc2268
    gcry_rijndael
    gcry_rmd160
    gcry_rsa
    gcry_seed
    gcry_serpent
    gcry_sha1
    gcry_sha256
    gcry_sha512
    gcry_tiger
    gcry_twofish
    gcry_whirlpool
    gettext
    gfxmenu
    gfxterm
    gfxterm_background
    gzio
    halt
    hashsum
    help
    hfsplus
    iso9660
    jfs
    jpeg
    keystatus
    linux
    loadenv
    loopback
    ls
    lsefi
    lsefimmap
    lsefisystab
    lssal
    luks
    luks2
    lvm
    mdraid09
    mdraid1x
    memdisk
    minicmd
    normal
    ntfs
    part_apple
    part_gpt
    part_msdos
    password_pbkdf2
    png
    probe
    raid5rec
    raid6rec
    reboot
    regexp
    search
    search_fs_file
    search_fs_uuid
    search_label
    serial
    sleep
    smbios
    squash4
    test
    true
    video
    xfs
    zfs
    zfscrypt
    zfsinfo
"

HOST_ARCH=$(dpkg --print-architecture)

mkdir -p "/build/outside/bin/"

# We need to place the configuration in a memdisk as this is required for the scripting
# capabilities to work. Without them, we could not use `if` and other commands.
mkfs.msdos -C "/build/memdisk.fat" "${MEMDISK_SIZE}"
mcopy -i "/build/memdisk.fat" "/build/outside/cfg/grub-zero.grub.cfg" ::grub.cfg
mdir -/ -i "/build/memdisk.fat"

"/opt/grub-${HOST_ARCH}/bin/grub-mkimage" \
    -O arm64-efi \
    -o /build/outside/bin/BOOTAA64.efi \
    -d /opt/grub-arm64/lib/grub/arm64-efi \
    -p "/EFI/rugpi" \
    -c /build/outside/cfg/grub-bootstrap.grub.cfg \
    -m "/build/memdisk.fat" \
    ${GRUB_MODULES}

"/opt/grub-${HOST_ARCH}/bin/grub-mkimage" \
    -O x86_64-efi \
    -o /build/outside/bin/BOOTX64.efi \
    -d /opt/grub-amd64/lib/grub/x86_64-efi \
    -p "/EFI/rugpi" \
    -c /build/outside/cfg/grub-bootstrap.grub.cfg \
    -m "/build/memdisk.fat" \
    ${GRUB_MODULES} \
    cpuid play tpm
