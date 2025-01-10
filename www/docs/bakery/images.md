---
sidebar_position: 5
---

# Images

As part of the project configuration you can define _system images_. A system image is a binary blob containing a partition table together with the required partitions and filesystems to boot a fully-functioning system. Images are typically used to provision new systems by flashing them onto a system's storage, e.g., integrated eMMC flash memory or an SSD.

Images are defined in the `images` section of the project configuration. Here is an example taken from the [Debian template](https://github.com/silitics/rugpi/tree/main/bakery/templates/debian-grub-efi):

```toml
[images.customized-arm64]
layer = "customized"
architecture = "arm64"
target = "generic-grub-efi"
```

This will instruct Rugix Bakery to build an image named `customized-arm64` based on the `customized` layer for `arm64` and using `generic-grub-efi` as a _device target_ (the image will work on any EFI-compatible hardware). 

Every image is based on a layer providing, among other things, the root filesystem for the image. In addition, an [architecture](./#architectures) must be specified. The `target` setting is optional and used to select some defaults for supported devices.


## Targets

When building a system image with Rugix Bakery, you can specify a *target* that is appropriate for your device. Targets typically support a whole family of devices and are categorized into *generic*, *specific*, and *unknown* targets.

- **Generic targets** are based on some standardized booting mechanism, such as [UEFI](https://en.wikipedia.org/wiki/UEFI) or [EBBR](https://github.com/ARM-software/ebbr).
Generic targets are suitable for any device that supports the respective booting mechanism.
- **Specific targets**, on the other hand, are limited to a certain family of devices.
They come with all necessary device-specific configurations resulting in a bootable image that works out-of-the-box.
- **Unknown targets** are for devices that do not conform to a standardized booting mechanism and are not specifically supported by Rugix Bakery. Using unknown targets allows building images for unsupported devices; however, these images may require additional device-specific modifications to become bootable.

The target for an image is set by the `target` property in the image declaration.

For supported devices and the required targets, checkout the documentation on [Supported Devices](/devices).

Currently, Rugix Bakery supports the following targets:

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

## Layouts (Experimental)

:::warning
**This is an experimental feature. We may introduce breaking changes in minor versions.**
:::

Usually, you do not need to worry about image layouts as Rugix Bakery automatically selects a suitable layout based on the device target.
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

The image layout is specified in the `layout` section. For details, we refer to the [project configuration reference](./projects.mdx#project-configuration).