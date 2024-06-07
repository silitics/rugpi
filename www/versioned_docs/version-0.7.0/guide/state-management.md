---
sidebar_position: 4
---

# State Management

Managing and persisting mutable system state can be quite challenging.
In particular, if state needs to be migrated by updates.
To facilitate robust state management and improve a system's resiliency, Rugpi has a few tricks up its sleeves.

First and foremost, the system's boot and root partition are always mounted read-only preventing any accidental modification or corruption of the system.
However, as a read-only root filesystem is difficult to deal with, a writable overlay is used yielding a writeable root filesystem.
Rugpi Ctrl can be configured to keep this overlay across reboots or discard it.
By default, the overlay is discarded such that the system is always booted fresh as stored in the root partition.
In addition, by discarding the overlay on each reboot, accidental state, which would get lost by an update, is discovered early as it is being erased by a simple reboot.

To enable persistency of the overlay, use the following option in `ctrl.toml`:

```toml title="ctrl.toml"
overlay = "persist"
```

To change the default size of the system partition use the `system_size` option in `ctrl.toml`.

Note that you must use a recipe to install `ctrl.toml` to `/etc/rugpi` in the image.

**⚠️ Cautionary Note:** Enabling persistency of the overlay, while convenient for certain use cases, requires careful consideration. Indiscriminate persistency can easily lead to system corruption due to accidental state or data loss during updates. We recommend selectively persisting state (see below) to avoid potential issues.

For development purposes, you can also force persistency of the overlay at runtime.
To this end, run:

```shell
rugpi-ctrl state overlay force-persist true
```

To disable force persistency of the overlay, run:

```shell
rugpi-ctrl state overlay force-persist false
```

Note that this will discard the overlay and thereby any modifications which have been made with persistency set to `true`.

## Selective Persistent State

State which should persist across updates (and reboots) must be explicitly managed.
In general, any data stored in `/var/rugpi/state` survives updates and reboots.
However, storing data there is not always possible or convenient.
Hence, Rugpi Ctrl can be configured to use `--bind` mounts to persist certain parts of the filesystem on the data partition.

For instance, to persist the home directory of the `root` user, the following configuration file is used:

```toml title="/etc/rugpi/state/root-home.toml"
[[persist]]
directory = "/root"
```

This instructs Rugpi Ctrl to `--bind` mount a persistent and writable directory to `/root` thereby preserving the entire home directory of the `root` user.
Note that the `persist-root-home` recipe automatically adds such a file.

Rugpi is similar to Docker in that regard.
A system should be disposable like a Docker container while any important state, that needs to persist, should be stored in an explicitly declared way (like volumes in Docker).

### Factory Reset

Because the state is managed by Rugpi Ctrl, a factory reset is simply done with:

```shell
rugpi-ctrl state reset
```

This command will reboot the system and throw away any state replacing it with factory defaults.
These factory defaults are taken from the system image.
If you persist a directory and the directory exists in the system image, it is copied.

## (Planned) Exporting and Importing

**🚧 This feature is planned but has not been implemented yet! 🚧**

Export all state and later import it.

```shell
rugpi-ctrl state export <file name>.tar.xz
```

```shell
rugpi-ctrl state import <file name>.tar.xz
```

## (Planned) State Profiles

**🚧 This feature is planned but has not been implemented yet! 🚧**

Sometimes it can be beneficial to support multiple _profiles_ with different state.

```shell
rugpi-ctrl state list
```

```shell
rugpi-ctrl state create <profile name>
```

```shell
rugpi-ctrl state switch <profile name>
```

## (Planned) Hardware Reset Button

**🚧 This feature is planned but has not been implemented yet! 🚧**

Should a device hang in a boot loop or otherwise malfunction because of corrupted state, a hardware reset button may be a convenient way for users to reset a system in the field, without requiring an expensive service technician to come out or sending the device to a repair center.
For this purpose, Rugpi Ctrl can be configured to check whether a reset button connected to one of the GPIO pins of the Raspberry Pi is pressed.
If this is the case and the button is hold for a specific amount of time, a factory reset can be automatically performed with the option to preserve the corrupted state for later (remote) recovery.
Likewise, another button could be programmed to initiate a reboot to the spare system (see [Over-the-Air Updates](./over-the-air-updates)), increasing the resiliency even further.

## Implementation Details

The state management is implemented by a custom init process which performs the necessary mounts and sets up the root filesystem prior to invoking Systemd.
For further details, checkout [the implementation](https://github.com/silitics/rugpi/blob/main/crates/rugpi-ctrl/src/init.rs).
