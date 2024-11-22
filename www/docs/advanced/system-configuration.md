# System Configuration

This page provides an in-depth guide to configuring a Rugpi system with OTA update capabilities for users who require fine-grained control or are migrating from a different OTA solution.
For Rugpi-native systems based on images built with Rugpi Bakery, Rugpi will typically recognize system configurations automatically.
However, for those needing to customize or manage specific aspects, the following sections detail the configuration concepts and available options.

System configuration is managed through the _system configuration file_ located at `/etc/rugpi/system.toml`.

Throughout this documentation, _root device_ refers to the parent block device of the block device mounted at `/` or Rugpi's system partition mount point `/run/rugpi/mounts/system`, with the latter taking priority if present.

## Config and Data Partitions

The _config partition_ and _data partition_ serve as core storage elements for a device.
The config partition, usually the first on a device, holds critical settings like bootloader configurations and optional device-specific parameters.
Meanwhile, the data partition retains persistent data, including user and system state, which is preserved across updates.

These partitions are defined in the `config-partition` and `data-partition` sections of the system configuration file, respectively.
Partitions can be specified either via the `device` setting, which points to a specific block device, or via the `partition` setting, which identifies a root device partition by its number.

Example configuration:

```toml
[config-partition]
# The config partition is `/dev/sda1`.
device = "/dev/sda1"

[data-partition]
# The 7th partition of the root device is the data partition.
partition = 7
```

Setting `disabled = true` allows the config and data partitions to be disabled individually.
The config partition is required for most bootloader integrations and for bootstrapping, while the data partition is required for state management.
If the partitions are not managed and mounted by Rugpi itself, their path can be specified via the `path` setting.

## Slots and Boot Groups

Rugpi's OTA update mechanism uses _slots_ to flexibly handle different OTA scenarios and requirements.
Typically, a _slot_ corresponds to a device partition where updates can be applied.
Each slot has a name and a _type_.
Currently, the only supported slot type is `block`, corresponding to a block device. The block device of a `block` slot can be specified either explicitly via the `device` setting or by a root device partition number via the `partition` setting.

Example configuration for an A/B update setup with four slots:

```toml
[slots.boot-a]
type = "block"
device = "/dev/sda2"

[slots.boot-b]
type = "block"
device = "/dev/sda3"

[slots.system-a]
type = "block"
partition = 4

[slots.system-b]
type = "block"
partition = 5
```

Slots are grouped into _boot groups_, which are sets of related slots into which the system can boot via a bootloader integration.
Each boot group has a name and a _slot mapping_ that defines aliases for slot names.

Example configuration for boot groups based on the A/B configuration given above:

```toml
[boot-groups.a]
slots = { boot = "boot-a", system = "system-a" }

[boot-groups.b]
slots = { boot = "boot-b", system = "system-b" }
```

An update artifact can carry updates for multiple slots, identified by their name or boot group alias.
Updates are installed to a designated boot group where the aliases are used to identify slots.
For example, if an update includes `boot` and `system` slots, they will be installed to the appropriate A or B partitions based on the selected boot group.
Boot groups are also used to prevent updates of _active_ slots, where an active slot is one referenced by the currently booted boot group.

## Boot Flow

Bootloader integrations are referred to as _boot flows_.
Multiple boot flows may exist for the same bootloader, allowing Rugpi to adapt to different environments and serve as a drop-in replacement for other OTA solutions.

Boot flows are configured via the `boot-flow` section, with the `type` setting indicating the boot flow type.

Currently, Rugpi supports the following boot flows:

- `tryboot`: Uses Raspberry Pi's `tryboot` mechanism (A/B setups only).
- `u-boot`: Uses an U-Boot environment file to switch between partitions (A/B setups only).
- `grub-efi`: Uses a Grub environment file to switch between partitions (A/B setups only).
