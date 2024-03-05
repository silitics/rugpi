# Changelog

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
