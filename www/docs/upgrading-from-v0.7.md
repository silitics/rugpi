---
sidebar_position: 45
---

# Upgrading from v0.7

Rugpi has been renamed to Rugix and gained a lot of new functionality.
As part of the transition, we also cleanly separated Rugix Ctrl from Rugix Bakery, so that they can be used independently.
Here is what you need to do to upgrade to Rugix v0.8:

- You need to replace “Rugpi” with “Rugix” everywhere (including in any environment variables).
- The overlay configuration has been moved into [`/etc/rugix/state.toml`](./ctrl/state-management.mdx#overlay-configuration).
- The system size configuration has been moved into [`/etc/rugix/bootstrapping.toml`](./ctrl/bootstrapping.mdx#default-layout).
- The output of `rugix-ctrl system info` has changed. If you integrate with Rugix Ctrl, use the new JSON output.
- The `images` section in `rugpi-bakery.toml` has been superseded by a `systems` section in `rugix-bakery.toml`.
- Updates should now be delivered as Rugix update bundles.
