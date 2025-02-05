---
sidebar_position: 45
---

# Upgrading from v0.7

Rugpi has been renamed to Rugix and gained a lot of new functionality.
As part of the transition, we also cleanly separated Rugix Ctrl from Rugix Bakery, so that they can be used independently.
Here is what you need to do to upgrade to Rugix.

Generally, you need to replace “Rugpi” with “Rugix” everywhere (including in any environment variables and paths).

#### Rugix Ctrl

- The overlay configuration has been moved into [`/etc/rugix/state.toml`](./ctrl/state-management.mdx#overlay-configuration).
- The system size configuration has been moved into [`/etc/rugix/bootstrapping.toml`](./ctrl/bootstrapping.mdx#default-layout).
- The output of `rugix-ctrl system info` has changed. If you integrate with Rugix Ctrl, use the new JSON output.
- Updates should now be delivered as Rugix update bundles instead of system images.
- The deprecated option `--stream` of `rugpi-ctrl update install` has been removed.
- The option `--no-reboot` has been removed in favor of `--reboot no`.

#### Rugix Bakery

- The `images` section in `rugpi-bakery.toml` has been superseded by a `systems` section in `rugix-bakery.toml`.
- The environment variable `RUGPI_BUNDLE_DIR` has been renamed to `RUGIX_LAYER_DIR`.