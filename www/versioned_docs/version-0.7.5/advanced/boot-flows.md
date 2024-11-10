---
sidebar_position: 4
---

# Boot Flows

A *boot flow* provides the base mechanism to switch between the A and B system, e.g., after installing an update.
To this end, it must implement two primitive operations: (i) rebooting to the spare system once and (ii) setting the default system.
Boot flows are typically implemented on top of a bootloader and Rugpi offers out-of-the-box integrations with popular bootloaders:

- [Raspberry Pi's `tryboot` Mechanism](https://www.raspberrypi.com/documentation/computers/config_txt.html#example-update-flow-for-ab-booting)
- [U-Boot](https://docs.u-boot.org/en/latest/) (popular on single board computers, GPL-2.0)
- [Grub](https://www.gnu.org/software/grub/) (well-established standard option, GPL-3.0)
<!-- - [Systemd Boot](https://www.freedesktop.org/software/systemd/man/latest/systemd-boot.html) (newer alternative to Grub, GPL-2.0) -->

The boot flow for an image is chosen based on the target of the image as configured in `rugpi-bakery.toml`.
Depending on the boot flow, Rugpi will automatically select an appropriate partitioning scheme for the image and system.

## Supported Boot Flows

We will now discuss the supported boot flows in more detail.

### Tryboot

```
MBR =============================== Image
     1: config    FAT32  256M
     2: boot-a    FAT32  128M  (*)
     3: boot-b    FAT32  128M
     5: system-a               (*)
    =============================== System
     6: system-b
     7: data      EXT4   ....
```

The `tryboot` boot flow works almost as described in [Raspberry Pi's documentation on the `tryboot` mechanism](https://www.raspberrypi.com/documentation/computers/config_txt.html#example-update-flow-for-ab-booting).
Instead of reading the device tree `tryboot` flag, it compares the booted partition with the default stored in `autoboot.txt`.

This boot flow also allows updating the `config.txt` file as well as the device tree files.

### U-Boot

```
MBR =============================== Image
     1: config    FAT32  256M
     2: boot-a    FAT32  128M  (*)
     3: boot-b    FAT32  128M
     5: system-a               (*)
    =============================== System
     6: system-b
     7: data      EXT4   ....
```

Rugpi supports upstream U-Boot, i.e., it does not require any patches to it.
Rugpi achieves this by using U-Boot boot scripts to control the boot process.
To this end, it relies on two environment files, `bootpart.default.env` and `boot_spare.env`, placed in the first partition, i.e., the `config` partition, of the boot drive.
The file `bootpart.default.env` sets the `bootpart` variable either to `2` or to `3` indicating the default boot partition (`boot-a` or `boot-b`).
The file `boot_spare.env` sets the `boot_spare` variable either to `1` or to `0` indicating whether the spare or default partition should be booted, respectively.
These files can then be used by U-Boot boot scripts to control the boot process.
In addition, there are the files `boot_spare.enabled.env` and `boot_spare.disabled.env` for overwriting the `boot_spare.env` file, e.g., to reset `boot_spare` to `0`.

A typical U-Boot boot script would proceed as follows:

1. Load `bootpart.default.env` and `boot_spare.env`.
2. If `boot_spare` is set to `1`, invert `bootpart`.
3. if `boot_spare` is set to `1`, overwrite `boot_spare.env` with `boot_spare.disabled.env`.
4. Proceed booting from the respective partition.

The reference implementation for Raspberry Pi uses two boot scripts, one first stage boot script on the config partition and a second stage boot script on the respective boot partition.
The first stage follows the steps outlined above and then loads the second stage boot script.
This has the advantage that the second stage script can be updated in a fail-safe way.

For further details, we refer to the reference [boot scripts](https://github.com/silitics/rugpi/tree/main/boot/u-boot/scripts) for Raspberry Pi.

### Grub (EFI)

```
GPT =============================== Image
     1: config    FAT32  256M
     2: boot-a    EXT4   128M  (*)
     3: boot-b    EXT4   128M
     5: system-a               (*)
    =============================== System
     6: system-b
     7: data      EXT4   ....
```

Follows a similar approach to U-Boot, using Grub boot scripts and environment blocks.

### Systemd Boot

:::warning

**Not implemented yet!**

:::

```
GPT =============================== Image
     1: EFI       FAT32  512M  (*)
     2: system-a               (*)
    =============================== System
     3: system-b
     4: data      EXT4   ....
```

Uses the [Boot Loader Interface](https://systemd.io/BOOT_LOADER_INTERFACE/) for A/B updates by writing to the following EFI variables:

- `LoaderEntryDefault-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f` (default entry)
- `LoaderEntryOneShot-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f` (oneshot entry)

In contrast to the other boot flows there are no separate boot partitions.

## Runtime Detection

Rugpi detects the boot flow of a system dynamically at runtime by inspecting the first partition:

1. If a file `autoboot.txt` exists, then the boot flow is `tryboot`.
2. If a file `bootpart.default.env` exists, then the boot flow is `u-boot`.
3. If a file `rugpi/grub.cfg` and a directory `EFI` exist, then the boot flow is `grub-efi`.
<!-- 4. If a file `loader/loader.conf` exists, then the boot flow is `systemd-boot`. -->

This information is used for repartitioning the root drive and interpreting updates.

## On Atomicity of Commits

Note that commits are the only critical operation because they modify the default partition set.
This is usually done by temporarily remounting the bootloader configuration partition such that it is writeable and then replacing some files.
As the filesystem is FAT32, the automitcity of this operation cannot be guaranteed.
Still, Rugpi Ctrl does its best by first creating a new file and then replacing the old one with the new one by renaming it, and, the Linux kernel does guarantee atomicity for renaming.
However, should the system crash during this process, the FAT32 filesystem may still be corrupted.
We think that this is an acceptable risk as the likelihood of it happening is very low and any alternatives, like swapping the MBR, may be problematic for other reasons.[^4]

[^4]: If you have any suggestions, please share them with us.