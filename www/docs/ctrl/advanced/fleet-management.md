---
sidebar_position: 1
---

# Fleet Management

Rugix Ctrl provides a reliable foundation for OTA updates and state management.
To manage a fleet of devices remotely and deliver updates to devices, Rugix Ctrl integrates will with existing off-the-shelf fleet management solutions.

:::tip
As Rugix Ctrl is independent from a fleet management solution, it avoids vendor lock-in.
When using Rugix Ctrl, **you can switch between different fleet management solutions at anytime** and continue updating your existing fleet.
It also allows you to chose a fleet management provider based on the needs and requirements of your application.
:::

Currently, there are ready-made integrations with [thin-edge.io](https://thin-edge.io/) and [Mender](https://mender.io/).
For other fleet management solutions, you can develop your own integration or [contract Silitics](/commercial-support) to develop an integration for you.


## Thin-edge.io

[Thin-edge.io](https://thin-edge.io/) is an open-source, cloud-agnostic IoT framework designed for resource constraint devices.
It provides an abstraction layer to interface with different providers of IoT management solutions such as [Cumulocity IoT](https://www.cumulocity.com/guides/concepts/introduction/), [Azure IoT](https://azure.microsoft.com/en-us/solutions/iot), and [AWS IoT](https://aws.amazon.com/iot/).
Thin-edge.io officially supports Rugix Bakery as well as Rugix Ctrl.
That is, integrating Thin-edge.io into your system is straightforward with ready-made recipes for Rugix Bakery.
Those recipes will also include an integration layer for Rugix Ctrl so that you can deploy updates without any further configuration.
To learn more, check out the [Thin-edge.io Rugix reference repository](https://github.com/thin-edge/tedge-rugpi-image).

## Mender

Open-source Mender support for Rugix Ctrl and Rugix Bakery is provided by [Silitics](https://silitics.com), check out the [Mender Rugix reference repository](https://github.com/silitics/rugpi-template-mender).
The Mender integration consists in recipes for Rugix Bakery that will install Mender's client as well as a Mender update module to install updates via Mender with Rugix Ctrl.
Note that Mender also offers their own update installation mechanism, which is part of the Mender client.
When you use Mender with Rugix Ctrl, you will not be using this mechanism but Rugix Ctrl instead.

**Mender vs. Rugix.**
Here is a rough comparison of Mender and Rugix.
Note that Rugix does not compete with Mender's fleet management solution.
In fact, **Rugix is perfectly compatible with Mender (as stated above) and we encourage you to consider using them together**.
Thus, this comparison concerns only the update installation mechanism itself and its build system support.

For building Debian-based systems, Mender provides a [conversion approach](https://docs.mender.io/operating-system-updates-debian-family/convert-a-mender-debian-image), assuming that you already built your Debian image, which you then convert for usage with Mender.
As Mender only provides a conversion from an existing image, you need a tool like Rugix Bakery in any case to prepare the image that you feed into the conversion.[^golden-image]
With Rugix Bakery, you get a modern, end-to-end workflow for building your image, based on a declarative system description and starting with a freshly bootstrapped Debian.
For building Yocto-based systems, Mender offers a ready-made Yocto integration for free whereas the Yocto integration for Rugix Ctrl is currently only available as a paid offering by Silitics.
To summarize, if you are building a Debian-based system, you should at least use Rugix Bakery.[^rugix-bakery-mender]
However, if you are building a Yocto-based system and do not want to buy into Rugix Ctrl's Yocto integration, then use Mender.

[^golden-image]: [Mender's documentation recommends](https://web.archive.org/web/20240815210840/https://docs.mender.io/operating-system-updates-debian-family/convert-a-mender-debian-image#recommended-workflow) that you boot an actual system with an image, make changes, and then extract the image from the running system. We strongly recommend not to use this so called _golden image_ workflow as it is a heavily manual process, making it impossible to reproduce and tedious to apply changes. You always have to manually update and integrate your application, which will lead to much less frequent updates with all the (security) implications that brings.

[^rugix-bakery-mender]: You can use it to build images and feed them into `mender-convert`, if you prefer Mender's update mechanism.

With regards to the features of the update mechanism itself, Mender's update mechanism and Rugix Ctrl are currently incomparable and you have to evaluate both for your concrete use case.
For instance, Mender's enterprise offering includes delta updates, which are not yet supported by Rugix Ctrl.
If this is something that you absolutely need, then either use Mender or [contract Silitics](/commercial-support).
Conversely, Mender's update mechanism does not offer anything comparable to Rugix Ctrl's managed state functionality and it does not support any other update configurations than A/B updates with two redundant system partitions.
Furthermore, Mender's update mechanism is tightly coupled to Mender's fleet management solution, which can be an advantage or a disadvantage.

If you are building on Raspberry Pi, note that Rugix Ctrl supports [Raspberry Pi's `tryboot` mechanism](https://www.raspberrypi.com/documentation/computers/config_txt.html#example-update-flow-for-ab-booting), which is the official way to do A/B updates on a Raspberry Pi.
Mender does not support the `tryboot` mechanism but relies on its U-Boot integration instead.
This means that Mender's support for newer Raspberry Pi models will typically be blocked by U-Boot support and lack behind Rugix's.
Furthermore, with the `tryboot` mechanism you can also update the boot partition, including changes to device tree overlays in `config.txt`, which you cannot do when using U-Boot.
Also, Mender's conversion approach so far does not work for 64-bit Raspberry Pi OS.[^mender-64-bit]
Hence, for Raspberry Pi, we definitely recommend using Rugix Ctrl instead of Mender.

[^mender-64-bit]: At the time of writing. For updates, see [this issue in Mender's issue tracker](https://northerntech.atlassian.net/browse/MEN-5634).