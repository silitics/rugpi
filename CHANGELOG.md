# Changelog

## Version 0.7.0

New features:

- Official support for Alpine Linux and Debian.
- Support for EFI systems and integration with Grub.
- Configurable image layouts.

Breaking changes to the image building pipeline:

- The `boot_flow` option has been superseded by `target`.
- The `include_firmware` option has been removed. To include a firmware update for Raspberry Pi, use the `core/rpi-include-firmware` recipe.
- The following recipes have been renamed:
    - `core/raspberrypi` => `core/rpi-raspios-setup`
    - `core/pi-cleanup` => `core/rpi-raspios-cleanup`
    - `core/apt-cleanup` => `core/pkg-cleanup` (also supports `apk` now)
    - `core/apt-update` => `core/pkg-cleanup` (also supports `apk` now)
    - `core/apt-upgrade` => `core/pkg-upgrade` (also supports `apk` now)
- The following recipes have been removed:
    - `core/disable-swap` (now part of `rpi-raspios-cleanup` via parameter)

## Version 0.6.6

- Allow for deferred reboots into the spare partition set.
- Make streaming updates the default.

## Version 0.6.5

- Allow booting from external USB devices.
- Fix issues with Docker due to the usage of `chroot`.

## Version 0.6.4

- Allow `gz` compressed tarballs as base layer.
- Check root filesystem size when building an image.
- Ignore any files in the `layers` directory not ending with `.toml`.

## Version 0.6.3

- Allow local `.tar` files to be used as a layer.
- Patch `/etc/fstab` instead of overwriting it.

## Version 0.6.2

- Create directories when baking images.
- Ignore `.DS_Store` directories/files.

## Version 0.6.1

- Transparent decompression of XZ-compressed images.
- Switch to streaming updates in Rugpi Admin.

## Version 0.6.0

- Introduction of layers.
- Introduction of repositories.
- Backwards-incompatible changes to image building pipeline:
    + Layers instead of recipes in `rugpi-bakery.toml`.
    + Removal of default recipes. Recipes must be explicitly enabled.
    + Separate `images` sections in `rugpi-bakery.toml`.

## Version 0.5.0

- Support for all models of Raspberry Pi via U-Boot.
- Support for persisting the overlay by default.
- Experimental support for streaming updates.

## Pre-Releases (0.1 to 0.4)

- Initial experimental version.
