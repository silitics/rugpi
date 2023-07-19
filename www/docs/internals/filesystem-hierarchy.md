---
sidebar_position: 1
---

# Filesystem Hierarchy

Here is a quick reference for the filesystem hierarchy:

- `/`: Root filesystem (read-write with an overlay).
- `/boot`: Boot partition (read-only).
- `/run/rugpi/system`: System partition (read-only).
- `/run/rugpi/data`: Data partition (read-write).
- `/run/rugpi/state`: System state (read-write bind mounted).

The state directory `/run/rugpi/state` is bind mounted to the active state profile on the data partition.
Currently, this is always `/run/rugpi/data/state/default`, however, this will change in the future once the state profile feature lands.

🚧 **TODO**: The data partition should be better protected against accidental modifications outside of the state directory.

🚧 **TODO**: The size of the state should be limited to avoid filling the entire data partition.
We need some space for the system to boot.

🚧 **TODO**: Investigate the usage of `btrfs` instead of `ext4` for the data partition.
On an SD card (or other low-quality flash memory), we probably do not want to use `btrfs` due to write amplification increasing the wear on the cells and reducing their lifetime.

### Data Partition

The data partition in `/run/rugpi/data` has the following hierarchy:

- `overlay/work`: Overlay work directory.
- `overlay/root`: Overlay root directory.
- `state/default`: Default state directory.
- `state/<profile name>`: State directory for a given profile (planned).

## State

The state directory `/run/rugpi/state` has the following hierarchy:

- `overlay/a`: A system overlay state.
- `overlay/b`: B system overlay state.
- `persist`: Files and directories persisted with Rugpi Ctrl.
- `ssh`: Persistent SSH host keys.
- `machine-id`: Persistent `/etc/machine-id`.
- `app`: Persistent application data.

To persist the data of your application, use Rugpi Ctrl or the `app` directory.
