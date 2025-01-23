# Rugix Ctrl

_Rugix Ctrl_ is a powerful tool for robust over-the-air system updates and system state management.
It mitigates the risks associated with remote software updates in the field, **enabling you to ship the latest updates to your users with confidence**.

To set the stage, let's first focus on the things that could go wrong and the ideal features and properties of an update solution.

1. **Interrupted Updates:** If something interrupts the update process, such as an unplanned power outage, a partially installed update may leave the system in an inoperable state.
Therefore, a robust update solution must be _atomic_, ensuring that updates are either installed completely or not at all, always leaving the system in an operational state, no matter what happens.

2. **Uncertain Production Environment:** While extensive testing should be done prior to deploying any updates, replicating the exact production environment and conditions can be difficult.
An update that turns out to be incompatible with the particularities of the production environment under difficult to replicate conditions may leave the system in an inoperable state.
Therefore, a robust update solution must have the possibility for _on-device validation and rollback_ of updates.
If any problems are detected with an update on a particular device, a rollback to the previous, known-good version should be automatically triggered.

3. **Data Loss and Accidental State:** Whenever an update is installed, the existing state of a system must be handled carefully to ensure that no data is lost.
For instance, user settings and data stored on the device must be preserved.
At the same time, a system must be safeguarded against corruption by _accidental state_ that should not be kept, such as configuration files incompatible with the new version.
Therefore, a robust update solution must provide reliable _state management_ mechanisms.

4. **Cyber Attacks:** A malicious actor may try to compromise a device by installing a manipulated update.
If they succeed and gain access, they can use the device to further infiltrate the network it is attached to, gaining wide-spread access that can quickly lead to huge damages extending far beyond the functionality of the original device.
Therefore, an update solution should provide mechanisms to prevent manipulated updates from being installed.

Rugix Ctrl addresses these challenges by ensuring atomic updates, on-device validation with rollback capabilities, reliable state management, and protection against malicious updates.
By utilizing Rugix Ctrl, you can rest assured that your devices remain reliable, secure, and up-to-date, **allowing you to focus on delivering value to your users**.


## High-Level Overview

Rugix Ctrl is designed around _full system updates_.
That is, instead of updating individual parts of your system, such as individual libraries or just your application, Rugix Ctrl will always update the system as a whole.[^delta-updates] Full system updates are advantageous because they allow you to test all components together and ensure a consistent environment across devices.

[^delta-updates]: In case this leaves you concerned with the size of updates, we are actively working towards support for _delta updates_.
Delta updates allow for full system updates while minimizing the update size based on the actual changes over the old version.

Rugix Ctrl is an update installer and does not include any remote delivery mechanism for updates.
That is, it implements a mechanism for installing updates without prescribing the way in which updates find their way onto the device.
We believe that this separation is crucial as it avoids vendor lock-in and gives you the flexibility to integrate updates in the way that makes sense for your device.
To manage your devices and push updates to them, Rugix Ctrl is compatible with several [device management solutions](./advanced/device-management.md) and can be easily integrated into your application, e.g., by offering your users the ability to upload a firmware update in your own UI.

Rugix Ctrl ships as a binary, `rugix-ctrl`, running on your device.
This binary is used to query and manage the state of the system, to install updates, and to initiate rollbacks.
The state management functionality provided by Rugix Ctrl is completely optional and you can use Rugix Ctrl as an update installer only, if you wish.
In addition to `rugix-ctrl`, which runs on your device, Rugix Ctrl also provides a tool, `rugix-bundler`, to create _update bundles_.
Update bundles contain the actual data required to install an update, like filesystems and some meta information.
You can download pre-built binaries from [the Releases page of Rugix's Git repository](https://github.com/silitics/rugix/releases/).

:::tip
The easiest way to use Rugix Ctrl is with [Rugix Bakery](../bakery/index.md), a flexible and user-friendly build system for bespoke Linux distributions developed by the Rugix Project.
With Rugix Bakery, it is straightforward to integrate Rugix Ctrl into your system.
Furthermore, Rugix Bakery also includes `rugix-bundler` and can directly create update bundles for Rugix Ctrl.
:::

The following documentation focuses on the concepts behind Rugix Ctrl and its usage.
For the most part, it will assume that you already have a working integration of Rugix Ctrl into your system, which you get out-of-the-box when you use Rugix Bakery to build the system.
Among other things, the section [Advanced Topics](./advanced/) of this documentation covers how Rugix Ctrl can be integrated into and adapted for other systems.
Note that while being developed together, Rugix Ctrl can also be used completely independently of Rugix Bakery.
For commercial customers, Silitics, [the company behind Rugix](/open-source-commitment), offers a [Yocto](https://www.yoctoproject.org/) integration.