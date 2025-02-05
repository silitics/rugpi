---
sidebar_position: 3
---

# Boot Flows

A *boot flow* provides the base mechanism to switch between different boot groups, e.g., to realize an A/B update scheme.

Each boot flow must implement at least three operations:

- `set_try_next(group)`: Set the boot group to try first on the next boot (falling back to the current default).
- `get_default()`: Retrieve the current default boot group.
- `commit(group)`: Commit the currently active boot group.

Note that Rugix Ctrl will determine the active boot group itself.
The currently active boot group will be supplied to `commit` and the boot flow's commit operation should fail if it disagrees about what the active boot group is.

In addition, a boot flow may support the following operations:

- `pre_install(group)`: Runs before installing an update to the given group.
- `post_install(group)`: Runs after installing an update to the given group.


Installing an update to a boot group will trigger the following operations:

1. `pre_install(group)`
2. Installation of the update.
3. `post_install(group)`
4. `set_try_next(group)`
5. Reboot.

Rebooting with `--boot-group` or `--spare` will trigger the following operations:

1. `set_try_next(group)`
2. Reboot.

Committing an update will trigger the following operations:

1. `get_default()`
2. `commit(group)` only if the default and given group differ.

Note that `set_try_next` may or may not change the default boot group.
In any case, it must guaranteed that there is a (transitive) fallback to the current default, to make sure that a broken update will not leave the system in an inoperable state.


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
     2: boot-a    EXT4   256M  (*)
     3: boot-b    EXT4   256M
     4: system-a               (*)
    =============================== System
     5: system-b
     6: data      EXT4   ....
```

### Custom

`type = "custom"`

This boot flow allows you to write your own custom logic for controlling the boot process. An example may look as follows:

```toml title="/etc/rugix/system.toml"
[boot-flow]
type = "custom"
controller = "<path to your script>"
```

Your custom boot flow will be called with the name of the operation as the first argument.
If an operation takes a boot group as an argument, then the second argument will be the name of the boot group.
**We may add further, optional arguments in the future, hence, your boot flow should ignore any additional arguments.
We may also add further, optional operations, hence, your script should not do anything (except printing something on stderr), in case it receives an unknown operation as the first argument.**
Following these rules minimizes churn on your end.
The boot flow is expected to produce JSON output on stdout.

For now, all operations except `get_default` should simply return an empty JSON object on stdout and indicate success/failure, as usual, through the return code. The output of `get_default` is expected to have the following form:

```json
{ "group": "<name of the boot group>" }
```

:::tip
Custom boot flows can be used to realize a variety of different, more advanced update setups.
For instance, with custom boot flows you could implement a dead men's switch where systems have to be actively marked as *good* to prevent the bootloader from eventually falling back to a recovery system.
This makes the system even more robust in case something unexpected happens and the primary system stops working.
You can also use custom boot flows to migrate from other OTA solutions like Mender, RAUC, or SWUpdate.
If you need anything specific, Silitics, [the company behind Rugix](/commercial-support), can help you develop a custom boot flow that suits your needs or migrate your existing devices to Rugix Ctrl.
:::


### Systemd Boot

:::warning
**Support for Systemd Boot is not implemented yet.**
:::

```
GPT =============================== Image
     1: EFI       FAT32  512M  (*)
     2: system-a               (*)
    =============================== System
     3: system-b
     4: data      EXT4   ....
```

Support for Systemd Boot would use the [Boot Loader Interface](https://systemd.io/BOOT_LOADER_INTERFACE/) for A/B updates by writing to the following EFI variables:

- `LoaderEntryDefault-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f` (default entry)
- `LoaderEntryOneShot-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f` (oneshot entry)

In contrast to the other boot flows there would be no separate boot partitions.


## Automatic Runtime Detection

If no boot flow is configured, Rugix Ctrl will try to detect it dynamically at runtime by inspecting the config partition:

1. If a file `autoboot.txt` exists, then the boot flow is `tryboot`.
2. If a file `bootpart.default.env` exists, then the boot flow is `u-boot`.
3. If a file `rugpi/grub.cfg` and a directory `EFI` exist, then the boot flow is `grub-efi`.

In all other cases, runtime detection will fail.


## On Atomicity of Commits

Note that commits are the only critical operation because they modify the default boot group.
This is usually done by temporarily remounting the config partition such that it is writeable and then replacing some files.
As the filesystem is FAT32, the atomicity of this operation cannot be guaranteed.
Still, Rugpi Ctrl does its best by first creating a new file and then replacing the old one with the new one by renaming it, and, the Linux kernel does guarantee atomicity for renaming.
However, should the system crash during this process, the FAT32 filesystem may still be corrupted.
We think that this is an acceptable risk as the likelihood of it happening is extremely low and any alternatives, like swapping the MBR, may be problematic for other reasons.[^atomicity-suggestions]

[^atomicity-suggestions]: If you have any suggestions, please share them with us.