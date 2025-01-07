# Device Management

Rugpi provides the reliable foundation for building images, OTA updates, and state management.
To manage devices remotely, Rugpi integrates well with existing off-the-shelf device management solutions.
Currently, Rugpi provides ready-made integrations with [thin-edge.io](https://thin-edge.io/) and [Mender](https://mender.io/).
When building a device with Rugpi, you can also switch between those at any point in time.

## Thin-edge.io

[Thin-edge.io](https://thin-edge.io/) is an open-source, cloud-agnostic IoT framework designed for resource constraint devices.
It provides an abstraction layer to interface with different providers of IoT management solutions such as [Cumulocity IoT](https://www.cumulocity.com/guides/concepts/introduction/), [Azure IoT](https://azure.microsoft.com/en-us/solutions/iot), and [AWS IoT](https://aws.amazon.com/iot/).
Thin-edge.io officially supports Rugpi to build and deploy images.
To learn more, checkout the [thin-edge.io Rugpi reference repository](https://github.com/thin-edge/tedge-rugpi-image).

## Mender

Rugpi can be used to build images for use with [Mender's](https://mender.io/) device management solution.
Using Rugpi over [Mender's conversion approach](https://docs.mender.io/operating-system-updates-debian-family/convert-a-mender-debian-image) has the advantage that Rugpi's modern image building workflow and state management can be used.
In addition, it works for 64-bit Raspberry Pi OS, which `mender-convert` does not support,[^1] and on newer Raspberry Pi's the `tryboot` feature can be used to deploy updates to the boot partition, including changes to device tree overlays in `config.txt`.
To learn more, checkout the [Rugpi reference repository for the Mender integration](https://github.com/silitics/rugpi-template-mender).

[^1]: At the time of writing. For updates, see [this issue in Mender's issue tracker](https://northerntech.atlassian.net/browse/MEN-5634).
