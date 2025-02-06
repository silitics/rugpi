---
sidebar_position: 2
---

# Systems

A project can contain build definitions for multiple types of _systems_.
If you are developing an embedded device, it is very likely that you eventually need to support different versions or variants of the device simultaneously.
Furthermore, you may want to build images and other artifacts specifically for testing purposes with development tooling and testing keys or certificates.
You can do all that within a single Rugix Bakery project, allowing you to share build configurations between the different systems.

Systems are defined in the `systems` section of the project configuration.
Here is an example taken from the [Debian template](https://github.com/silitics/rugpi/tree/main/bakery/templates/debian-grub-efi):

```toml
[systems.customized-arm64]
layer = "customized"
architecture = "arm64"
target = "generic-grub-efi"
```

Each system has a _name_ (`customized-arm64` in the example), is based on a layer providing the root filesystem and Linux kernel for that system (`customized` in the example), and has a concrete CPU architecture (`arm64` in the example).
The `target` setting is optional and used to select defaults for supported devices (see below).

You can built different types of artifacts for a given system.
Currently, Rugix Bakery can build full system images and Rugix Ctrl update bundles.
Generally, artifacts are build with a command of the form:

```shell
./run-bakery bake <type> <system>
```

Here, `<type>` is the type of the artifact and `<system>` is the name of the system.

For instance, to build an image for a given system, run:

```shell
./run-bakery bake image <system>
```


## Targets

When declaring a system within the project configuration, you can specify a *target* that is appropriate for the respective device.
The primary purpose of targets is to build system images that can be directly booted on supported devices.
Targets typically support a whole family of devices and are categorized into *generic*, *specific*, and *unknown* targets.

- **Generic targets** are based on some standardized booting mechanism, such as [UEFI](https://en.wikipedia.org/wiki/UEFI) or [EBBR](https://github.com/ARM-software/ebbr).
Images built for a generic targets are suitable for any device that supports the respective booting mechanism.
- **Specific targets**, on the other hand, are limited to a certain family of devices.
Images built for specific targets come with all necessary device-specific configurations resulting in a bootable image that works out-of-the-box.
- **Unknown targets** are for devices that do not conform to a standardized booting mechanism and are not specifically supported by Rugix Bakery. Using unknown targets allows building images for unsupported devices; however, these images may require additional device-specific modifications to become bootable.

The target for a system is set by the `target` property in the system declaration.

For supported devices and the required targets, checkout the documentation on [Supported Devices](./devices/index.mdx).

Currently, Rugix Bakery supports the following targets:

- `generic-grub-efi`: A generic target that uses Grub as the bootloader and produces an image bootable on any EFI-compatible system.
This is the right target for commodity AMD64 and ARM64 hardware and VMs.
- `rpi-tryboot`: Raspberry Pi-specific target that uses [the `tryboot` feature of Raspberry Pi's firmware](https://www.raspberrypi.com/documentation/computers/config_txt.html#example-update-flow-for-ab-booting).
This is the right target for the Raspberry Pi 4 and 5 family of devices.
Note that for Raspberry Pi 4 a recent firmware is required.
- `rpi-uboot`: Raspberry Pi-specific target that uses [U-Boot](https://docs.u-boot.org/en/latest/).
This is the right target for older Raspberry Pi models.

Note that specific and generic targets result in images with a bootloader, however, to actually boot the operating system additional configurations may be required.
To this end, the following recipes can be used:

- `core/debian-grub-setup`: For Debian with `generic-grub-efi`.
- `core/alpine-grub-setup`: For Alpine with `generic-grub-efi`.
- `core/rpi-debian-setup`: For Debian with `rpi-tryboot`.
- `core/rpi-alpine-setup`: For Alpine with `rpi-tryboot`.
- `core/rpi-raspios-setup`: For Raspberry Pi OS with `rpi-tryboot` or `rpi-uboot`.


## Image Layouts

:::warning
**This is an experimental feature. We may introduce breaking changes to the configuration format in minor versions.**
:::

Usually, you do not need to worry about image layouts as Rugix Bakery automatically selects a suitable layout based on the system target.
For advanced use cases or devices that are not officially supported, Rugix Bakery also gives you the flexibility to configure how exactly the image should be built.
Currently, this is limited to creating partitions with optional filesystems.

#### Image Creation Process

The process of creating an image roughly works as follows:

1. Create the required filesystems from the layer.
2. Compute a partition table based on the specified image layout and filesystems.
3. Create an image and partition it according to the computed table.
4. Copy the prepared filesystems into the image.
5. Patch the boot configuration based on the target.

Supported partition tables:
- `gpt`: [GUID Partition Table](https://en.wikipedia.org/wiki/GUID_Partition_Table) (modern partition table, part of the UEFI standard)
- `mbr`: [MBR Partition Table](https://en.wikipedia.org/wiki/Master_boot_record) (legacy partition table, supported by almost any system)

Supported filesystems:
- `ext4`: [Ext4 Filesystem](https://en.wikipedia.org/wiki/Ext4)
- `fat32`: [FAT32 Filesystem](https://en.wikipedia.org/wiki/File_Allocation_Table)

The image layout is specified in the `image.layout` section. For details, we refer to the [project configuration reference](./projects.mdx#project-configuration).