Commands for installing Grub:

```shell
grub-install \
    --target i386-pc \
    --boot-directory boot /dev/loop0

EFI_TARGET="x86_64-efi"
grub-install \
    --target "${EFI_TARGET}" \
    --efi-directory boot \
    --boot-directory boot \
    --no-nvram \
    --bootloader-id rugpi \
    --removable \
    /dev/loop0 
```

To install Systemd Boot, we copy the EFI files manually:

```shell
cp /usr/lib/systemd/boot/efi/systemd-bootx64.efi boot/EFI/BOOT/BOOTX64.efi
cp /usr/lib/systemd/boot/efi/systemd-bootaa64.efi boot/EFI/BOOT/BOOTAA64.efi
cp /usr/lib/systemd/boot/efi/systemd-bootarm.efi boot/EFI/BOOT/BOOTARM.efi
```

