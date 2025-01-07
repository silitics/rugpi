---
sidebar_position: 5
---

# Images

The configuration file `rugpi-bakery.toml` contains a declaration for each image that can be built.

An *image declaration* has the following structure:

```typescript
type Image = {
    layer: string;
    architecture: "amd64" | "arm64" | "armv7" | "armhf" | "arm";
    target: "generic-grub-efi" | "rpi-tryboot" | "rpi-uboot";
    size?: string;
    layout?: ImageLayout;
}
```

Each image configuration has a mandatory `layer` property specifying the name of the layer containing the filesystems for the image, in particular, the root filesystem.
For further details on layers, checkout the [user guide's section on System Customization](./system-customization.md).
In addition, the configuration must specify an architecture and a target and may specify a size and an image layout.

## Architectures

Rugpi supports the following CPU architectures.

| Architecture | Description | Alpine | Debian | Raspberry Pi OS |
| ------------ | ----------- | ------ | ------ | --------------- |
| `amd64` | 64-bit x86 | `x86_64` | `amd64` | – |
| `arm64` | 64-bit ARMv8 | `aarch64` | `arm64` | `arm64` |
| `armv7` | 32-bit ARMv7 | `armv7` | `armhf` | – |
| `armhf` | 32-bit ARMv6 (Hard-Float) | `armhf` | – | `armhf` |
| `arm` | 32Tbit ARMv6 | – | `armel` | – |

Note that different distributions have different and sometimes inconsistent names for different CPU families.
For instance, what Debian calls `armhf` is called `armv7` for Alpine Linux and not the same as `armhf` for Raspberry Pi OS.

When building images the architecture reported by `uname -m` may not match the actual CPU and architecture of the device the image is intended for.
For instance, when building an `armhf` image based on Rasbperry Pi OS, the architecture reported by `uname -m` during the build process is `armv7l`, however, when running the image later on a non-ARMv7 board (e.g., Pi Zero or Pi 1), then the architecture will be `armv6l`.
We recommend always using the Rugpi architecture instead of `uname -m`.


## Targets

When building a system image with Rugpi, you need to specify a *target* that is appropriate for your device.
Targets typically support a whole family of devices and are categorized into *generic*, *specific*, and *unknown* targets.

- **Generic targets** are based on some standardized booting mechanism, such as [UEFI](https://en.wikipedia.org/wiki/UEFI) or [EBBR](https://github.com/ARM-software/ebbr).
Generic targets are suitable for any device that supports the respective booting mechanism.
- **Specific targets**, on the other hand, are limited to a certain family of devices.
They come with all necessary device-specific configurations resulting in a bootable image that works out-of-the-box.
- **Unknown targets** are for devices that do not conform to a standardized booting mechanism and are not specifically supported by Rugpi.
Using unknown targets allows building images for unsupported devices; however, these images usually require additional device-specific modifications to become bootable.

The target for an image is set by the `target` property in the image declaration.

For supported devices and the required targets, checkout the documentation on [Supported Devices](/devices).

Currently, Rugpi supports the following targets:

- `generic-grub-efi`: A generic target that uses Grub as the bootloader and produces an image bootable on any EFI-compatible system.
This is the right target for commodity AMD64 and ARM64 hardware or VMs.
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

## Image Layout (Experimental)

You can also specify an image layout.

:::warning

**This is an experimental feature. We may make breaking changes in minor versions.**

:::

Usually, you do not need to worry about image layouts as Rugpi automatically selects a suitable layout based on the target.
For advanced use cases or to adapt Rugpi to a device that is not officially supported, Rugpi also gives you the flexibility to configure how exactly the image should be built.
Currently, this is limited to creating partitions with an optional filesystem based on some root directory.

#### Image Creation Process

The process of creating an image roughly works as follows:

1. Compute a partition table based on the specified image layout.
2. Create an image and partition it according to the computed table.
3. Patch the boot configuration based on the target.
4. Create the filesystems based on the specified root directories.

Supported partition tables:
- `gpt`: [GUID Partition Table](https://en.wikipedia.org/wiki/GUID_Partition_Table) (modern partition table, part of the UEFI standard)
- `mbr`: [MBR Partition Table](https://en.wikipedia.org/wiki/Master_boot_record) (legacy partition table, supported by almost any system)

Supported filesystems:
- `ext4`: [Ext4 Filesystem](https://en.wikipedia.org/wiki/Ext4)
- `fat32`: [FAT32 Filesystem](https://en.wikipedia.org/wiki/File_Allocation_Table)

The image layout is specified within the `layout` section of the image configuration has the following structure:

```typescript
type Layout = {
    type: "gpt" | "mbr";
    partitions: Partition[];
}

type Partition = {
    type?: string;
    size?: string;
    filesystem?: "ext4" | "fat32";
    root?: string;
}
```