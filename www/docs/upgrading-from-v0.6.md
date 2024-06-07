---
sidebar_position: 40
---

# Upgrading from v0.6

To upgrade to v0.7, first update the `run-bakery` script with:

```shell
curl -O https://raw.githubusercontent.com/silitics/rugpi/v0.7/bakery/run-bakery && chmod +x ./run-bakery
```

Please also remove the old cache with:

```shell
rm -rf .rugpi
```

Here are the changes you need to make compared to version 0.6:

1. The image option `boot_flow` has been superseded by the `target` option. To use the new `target` option make the following changes depending your previous `boot_flow`:
    - `boot_flow = "u-boot"` ⇒ `target = "rpi-uboot"`
    - `boot_flow = "tryboot"` ⇒ `target = "rpi-tryboot"`
2. The `include_firmware` option has been replaced with the `core/rpi-include-firmware` recipe. Please use that recipe and remove the old option.
3. The following core recipes have been renamed:
    - `core/raspberrypi` ⇒ `core/rpi-raspios-setup`
    - `core/pi-cleanup` ⇒ `core/rpi-raspios-cleanup`
    - `core/apt-cleanup` ⇒ `core/pkg-cleanup` (also supports `apk` now)
    - `core/apt-update` ⇒ `core/pkg-upgrade` (also supports `apk` now)
    - `core/apt-upgrade` ⇒ `core/pkg-upgrade` (also supports `apk` now)
4. The following core recipes have been removed:
    - `core/disable-swap` (now part of `rpi-raspios-cleanup` via parameter)