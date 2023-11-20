---
sidebar_position: 0
---

# Supported Boards

Currently, Rugpi supports the following boards with the **64-bit** variant of Raspberry Pi OS:

| Pi 5 | Pi 4 | Pi 3 | Pi 2 | Pi Zero 2 | Pi Zero   | CM 4 | CM 3 |
|------|------|------|------|-----------|-----------|------|------|
| ✅   | ✅   | ❌   | ❌   | ❌        | ❌        | ✅   | ❌   |

**⚠️ Please also read the remarks for the respective boards bellow.**

Rugpi relies on the [`tryboot` feature of Raspberry Pi's bootloader](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html#fail-safe-os-updates-tryboot), which is only supported from Raspberry Pi 4 onwards.
There are plans to also support older boards without this feature.
For further details, see [issue #4](https://github.com/silitics/rugpi/issues/4).

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

[^1]: To prevent the EEPROM from being updated on each boot.