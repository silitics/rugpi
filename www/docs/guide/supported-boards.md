---
sidebar_position: 0
---

# Supported Boards

Currently, Rugpi supports 64-bit variants of Raspberry Pi and Raspberry Pi OS.

| Pi 5 | Pi 4 | Pi 3   | Pi 2 | Pi Zero 2 | Pi Zero | CM 4 | CM 3   |
| ---- | ---- | ------ | ---- | --------- | ------- | ---- | ------ |
| ✅   | ✅   | ✅[^1] | ❌   | 🤷‍♂️[^1]    | ❌      | ✅   | 🤷‍♂️[^1] |

✅ fully supported, 🤷‍♂️ in principle supported but untested, ❌ not supported

[^1]: Requires the U-Boot boot flow, for further details [read the docs](https://oss.silitics.com/rugpi/docs/guide/supported-boards).

**⚠️ Please also read the remarks for the respective boards bellow.**

Raspberry Pi OS releases based on Debian Bullseye and Bookworm are supported.

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

### Raspberry Pi 3, Zero 2, and CM3

You must enable the U-Boot [boot flow](../internals/boot-flows.md) in `rugpi-bakery.toml`:

```toml
boot_flow = "u-boot"
```

**⚠️ The U-Boot boot flow is experimental.**

[^1]: To prevent the EEPROM from being updated on each boot.
