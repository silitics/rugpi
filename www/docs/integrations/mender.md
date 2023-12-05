---
sidebar_position: 1
---

# Mender

Rugpi can be used to build images for use with [Mender's](https://mender.io/) device management solution.
Using Rugpi over [Mender's conversion approach](https://docs.mender.io/operating-system-updates-debian-family/convert-a-mender-debian-image) has the advantage that Rugpi's modern image building workflow and state management can be used.
In addition, it works for 64-bit Raspberry Pi OS, which `mender-convert` does not support,[^1] and on newer Raspberry Pi's the `tryboot` feature can be used to deploy updates to the boot partition, including changes to device tree overlays in `config.txt`.

To apply updates via Mender, a custom Mender update module is necessary.
At this time, the respective module is not open-source.
If you are interested, please [contact us](mailto:rugpi@silitics.com) so that we can send you the module.

[^1]: At the time of writing. For updates, see [this issue in Mender's issue tracker](https://northerntech.atlassian.net/browse/MEN-5634).
