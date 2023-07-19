---
sidebar_position: 2
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

To enable persistency of the overlay of the root filesystem, run:

```shell
rugpi-ctrl state overlay set-persist true
```

To disable persistency of the overlay of the root filesystem, run:

```shell
rugpi-ctrl state overlay set-persist false
```

Note that this will discard the overlay and thereby any modifications which have been made with persistency set to `true`.

## Selective Persistent State

State which should persist across updates (and reboots) must be explicitly managed.
In general, any data stored in `/var/rugpi/state` survives updates and reboots.
However, storing data there is not always possible or convenient.
Hence, Rugpi Ctrl can be configured to use `--bind` mounts to persist certain parts of the filesystem on the data partition.

For instance, to persist the data stored in a PostgreSQL database, the following configuration file is used:

```toml title="/etc/rugpi/state/postgresql.toml"
[[persist]]
directory = "/var/lib/postgresql/data"
```

This instructs Rugpi Ctrl to `--bind` mount a persistent and writable directory to `/var/lib/postgresql/data` thereby preserving the state of the PostgreSQL database.
Note that the `postgresql` recipe automatically adds such a file.

Rugpi is similar to Docker in that regard.
A system should be disposable like a Docker container while any important state, which needs to persist, should be stored in an explicitly declared way (like volumes in Docker).

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
