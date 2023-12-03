---
sidebar_position: 0
---

# Supported Boards

In principle, Rugpi supports all Raspberry Pi models.

| Pi 5 | Pi 4 | Pi 3   | Pi 2 v1.2 | Pi 2  | Pi 1   | Pi Zero 2 | Pi Zero | CM 4 | CM 3  | CM 1   |
| ---- | ---- | ------ | --------- | ----- | ------ | --------- | ------- | ---- | ----- | ------ |
| ✅   | ✅   | ✅[^1]  | ✅[^1]    | 🤷‍♂️[^1] | ✅[^1] | ✅[^1]     | ✅[^1]  | ✅   | 🤷‍♂️[^1] | 🤷‍♂️[^1] |

✅ fully supported, 🤷‍♂️ in principle supported but untested

[^1]: Requires the U-Boot boot flow.

**⚠️ Please also read the remarks for the respective boards bellow.**

Raspberry Pi OS releases based on Debian Bullseye and Bookworm are supported.

For 32-bit models and to build 32-bit images for 64-bit boards, you need to set the `architecture` in `rugpi-bakery.toml`:
```toml
architecture = "armhf"
```

To build 32-bit images, you also need to enable emulation of `armhf` in Docker:
```shell
docker run --privileged --rm tonistiigi/binfmt --install armhf
```

For `armhf`, note that the architecture reported by `uname -m` during the build process is `armv7l`, however, when running the image later on a non-ARMv7 board (e.g., Pi Zero or Pi 1), then the architecture will be `armv6l`.
Make sure that the binaries you install are compatible with the `armv6l` architecture, if you aim to deploy the image to these boards.

### Raspberry Pi 5

If you are using the quick-start template, please remove the option `include_firmware = "pi4"` from `rugpi-bakery.toml`.
This option will include the bootloader update for Raspberry Pi 4 (see bellow).
Updating the bootloader is not necessary for Raspberry Pi 5, as it already comes with the `tryboot` feature out-of-the-box.

### Raspberry Pi 4 and Compute Module 4

The bootloader version shipped with Raspberry Pi 4 and Compute Module 4 does not support the `tryboot` feature out-of-the-box.
To use Rugpi with these boards, the bootloader stored in the EEPROM must be updated to at least version `2023-05-11`.
For Compute Module 4, this requires `usbboot` (see [CM4's documentation for details](https://www.raspberrypi.com/documentation/computers/compute-module.html#flashing-the-bootloader-eeprom-compute-module-4) or check out [this blog post by Jeff Geerling](https://www.jeffgeerling.com/blog/2022/how-update-raspberry-pi-compute-module-4-bootloader-eeprom)).
For Raspberry Pi 4, you can set the `include_firmware = "pi4"` option in `rugpi-bakery.toml` to include the bootloader update in the image.
The bootloader will then be automatically updated when first booting the image.
Note that after the first boot, the automatic update will be disabled,[^1] i.e., you cannot take the SD card to another Raspberry Pi which does not yet have the update installed.

### Other Boards

For other boards than Pi 5, Pi 4, and CM 4, you must enable the U-Boot [boot flow](../internals/boot-flows.md) in `rugpi-bakery.toml`:

```toml
boot_flow = "u-boot"
```

**⚠️ The U-Boot boot flow is experimental and does not allow updating the firmware.**

[^1]: To prevent the EEPROM from being updated on each boot.
