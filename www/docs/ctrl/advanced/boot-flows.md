---
sidebar_position: 4
---

# Boot Flows

A *boot flow* provides the base mechanism to switch between different boot groups, e.g., to realize an A/B update scheme.[^rauc]

[^rauc]: This following design has been inspired by [RAUC](https://rauc.io/).

Each boot flow must implement at least three operations:

- `set_try_next(group)`: Set the boot group to try first on the next boot (falling back to the current default).
- `get_default()`: Retrieve the current default boot group.
- `commit(group)`: Commit the currently active boot group.

Note that Rugix Ctrl will determine the active boot group itself.
The currently active boot group will be supplied to `commit` and the boot flow's commit operation should fail if it disagrees about what the active boot group is.

In addition, a boot flow may support the following operations:

- `pre_install(group)`: Runs before installing an update to the given group.
- `post_install(group)`: Runs after installing an update to the given group.
- `remaining_attempts()`: Query the _remaining attempts_ of all boot groups.
- `get_status()`: Query the _status_ (`good`, `bad`, or `unknown`) of all boot groups.
- `mark_good(group)`: Mark the given boot group as _good_.
- `mark_bad(group)`: Mark the given boot group as _bad_.


Installing an update to a boot group will trigger the following operations:

1. `pre_install(group)`
2. `post_install(group)`
3. `set_try_next(group)`

Rebooting with `--boot-group` or `--spare` will trigger the following operations:

1. `get_status(group)`
2. `set_try_next(group)` only if the status is not _bad_.

Committing an update will trigger the following operations:

1. `get_default()`
2. `commit(group)` only if the default and given group differ.

Note that `set_try_next` may or may not change the default boot group.
In any case, it must guarantee that there is a (transitive) fallback to the current default.
Boot flows can choose to implement a mechanism whereby _remaining boot attempts_ are tracked per boot group.
The bootloader may then fallback to a different boot group once this number reaches zero.
This can be advantageous in the rare event that the default boot group experiences problems and a fallback to a recovery boot group should be triggered by the bootloader.
If the boot flow tracks attempts, `remaining_attempts()` should return the remaining attempts per boot group.
Marking a boot group as _good_ via `mark_good(group)` should reset the remaining attempts.
As a result, `mark_good` together with the counter can be used to implement a dead man's switch:
When the boot group is not marked good frequently, then the remaining attempts will reach zero eventually and the fallback into the recovery system will be triggered.
Note that Rugix Ctrl will not mark groups as good by itself.
Marking groups as good (or bad) is up to the application or boot flow.
Generally, a _good_ boot group is a boot group that the bootloader might boot and a _bad_ boot group is a boot group that the bootloader should not boot.
Note that most boot flows discussed in the following do neither implement tracking of remaining attempts nor of boot group status.


## Available Boot Flows

We will now discuss the available boot flows in more detail.

### Tryboot

`type = "tryboot"`

This boot flow is specific to Raspberry Pi 4 and newer models.

The `tryboot` boot flow works almost as described in [Raspberry Pi's documentation on the `tryboot` mechanism](https://www.raspberrypi.com/documentation/computers/config_txt.html#example-update-flow-for-ab-booting).
Instead of reading the device tree `tryboot` flag, it compares the booted partition with the default stored in `autoboot.txt`.

This boot flow assumes the following image and system layout:

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

This boot flow also allows updating the `config.txt` file as well as the device tree files.

### U-Boot

`type = "u-boot"`

Rugix Ctrl supports upstream U-Boot, i.e., it does not require any patches to it.
Rugix Ctrl achieves this by using U-Boot boot scripts to control the boot process.
To this end, it relies on two environment files, `bootpart.default.env` and `boot_spare.env`, placed in the first partition, i.e., the `config` partition, of the boot drive.
The file `bootpart.default.env` sets the `bootpart` variable either to `2` or to `3` indicating the default boot partition (`boot-a` or `boot-b`).
The file `boot_spare.env` sets the `boot_spare` variable either to `1` or to `0` indicating whether the spare or default partition should be booted, respectively.
In addition, there are the files `boot_spare.enabled.env` and `boot_spare.disabled.env` for overwriting the `boot_spare.env` file.

This boot flow assumes the following image and system layout:

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

`type = "grub-efi"`

This boot flow follows a similar approach to U-Boot, using Grub boot scripts and environment blocks.

For further details, we refer to the reference [boot scripts](https://github.com/silitics/rugpi/tree/main/boot/grub/cfg) used by Rugix Bakery.

This boot flow assumes the following image and system layout:

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

### Custom

`type = "custom"`

This boot flow allows you to write your own script for controlling the boot process. An example configuration may look as follows:

```toml title="/etc/rugix/system.toml"
[boot-flow]
type = "custom"
path = "<path to your script>"
```

Your script will be called with the name of the operation as the first argument.
The arguments for the operation are then fed via stdin as JSON to the script and the outputs are required to be written as JSON to stdout.

### Systemd Boot

:::warning
**Not implemented yet!**
This is blocked on directory slots in the system configuration.
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


## Automatic Runtime Detection

If no boot flow is configured, Rugix Ctrl will try to detect it dynamically at runtime by inspecting the config partition:

1. If a file `autoboot.txt` exists, then the boot flow is `tryboot`.
2. If a file `bootpart.default.env` exists, then the boot flow is `u-boot`.
3. If a file `rugpi/grub.cfg` and a directory `EFI` exist, then the boot flow is `grub-efi`.

## On Atomicity of Commits

Note that commits are the only critical operation because they modify the default partition set.
This is usually done by temporarily remounting the config partition such that it is writeable and then replacing some files.
As the filesystem is FAT32, the atomicity of this operation cannot be guaranteed.
Still, Rugpi Ctrl does its best by first creating a new file and then replacing the old one with the new one by renaming it, and, the Linux kernel does guarantee atomicity for renaming.
However, should the system crash during this process, the FAT32 filesystem may still be corrupted.
We think that this is an acceptable risk as the likelihood of it happening is extremely low and any alternatives, like swapping the MBR, may be problematic for other reasons.[^atomicity-suggestions]

[^atomicity-suggestions]: If you have any suggestions, please share them with us.