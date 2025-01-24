---
sidebar_position: 1
---

# Filesystem Hierarchy

Here is a quick reference of different directories used by Rugix Ctrl:

- `/`: Root filesystem (read-write with an overlay).
- `/run/rugix/mounts/config`: Config partition (usually read-only).
- `/run/rugix/mounts/system`: System partition (read-only).
- `/run/rugix/mounts/data`: Data partition (read-write).
- `/run/rugix/state`: System state (read-write bind mounted).

The state directory `/run/rugix/state` is bind mounted to the active state profile on the data partition.
Currently, this is always `/run/rugix/data/state/default`, however, this will change in the future once the state profile feature lands.

🚧 **TODO**: The data partition should be better protected against accidental modifications outside of the state directory.

🚧 **TODO**: The size of the state should be limited to avoid filling the entire data partition.
We need some space for the system to boot.

🚧 **TODO**: Investigate the usage of `btrfs` instead of `ext4` for the data partition.
On an SD card (or other low-quality flash memory), we probably do not want to use `btrfs` due to write amplification increasing the wear on the cells and reducing their lifetime.

### Data Partition

The data partition in `/run/rugix/data` has the following hierarchy:

- `overlay/work`: Overlay work directory.
- `overlay/root`: Overlay root directory.
- `state/default`: Default state directory.
- `state/<profile name>`: State directory for a given profile (planned).

## State

The state directory `/run/rugix/state` has the following hierarchy:

- `overlay/<group name>`: System overlay for the respective boot group.
- `persist`: Files and directories persisted with Rugix Ctrl.
- `ssh`: Persistent SSH host keys.
- `machine-id`: Persistent `/etc/machine-id`.
- `app`: Persistent application data (unmanaged).
