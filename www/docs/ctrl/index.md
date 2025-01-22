# Rugix Ctrl

_Rugix Ctrl_ is a powerful tool for robust over-the-air system updates and system state management. It mitigates the risks associated with remote software updates in the field, **enabling you to ship the latest updates to your users with confidence**.

To set the stage, let's first focus on the things that could go wrong and the ideal features and properties of an update solution.

1. **Interrupted Updates:** If something interrupts the update process, such as an unplanned power outage, a partially installed update may leave the system in an inoperable state. Therefore, a robust update solution must be _atomic_, ensuring that updates are either installed completely or not at all, always leaving the system in an operational state, no matter what happens.

2. **Uncertain Production Environment:** While extensive testing should be done prior to deploying any updates, replicating the exact production environment and conditions can be difficult. An update that turns out to be incompatible with the particularities of the production environment under difficult to replicate conditions may leave the system in an inoperable state. Therefore, a robust update solution must have the possibility for _on-device validation and rollback_ of updates. If any problems are detected with an update on a particular device, a rollback to the previous, known-good version should be automatically triggered.

3. **Data Loss and Accidental State:** Whenever an update is installed, the existing state of a system must be handled carefully to ensure that no data is lost. For instance, user settings and data stored on the device must be preserved. At the same time, a system must be safeguarded against corruption by _accidental state_ that should not be kept, such as configuration files incompatible with the new version. Therefore, a robust update solution must provide reliable _state management_ mechanisms.

4. **Cyber Attacks:** A malicious actor may try to compromise a device by installing a manipulated update. Therefore, an update solution should provide mechanisms to prevent manipulated updates from being installed.

Rugix Ctrl addresses these challenges by ensuring atomic updates, on-device validation with rollback capabilities, reliable state management, and protection against malicious updates. By utilizing Rugix Ctrl, you can rest assured that your devices remain reliable, secure, and up-to-date, **allowing you to focus on delivering value to your users**.


## High-Level Overview

Rugix Ctrl is designed around _full system updates_. That is, instead of updating individual parts of your system, such as individual libraries or just your application, Rugix Ctrl will always update the system as a whole.[^delta-updates] Full system updates are advantageous because they allow you to test all components together and ensure a consistent environment across devices.

[^delta-updates]: In case this leaves you concerned with the size of updates, we are actively working towards support for _delta updates_. Delta updates allow for full system updates while minimizing the update size based on the actual changes over the old version.

Rugix Ctrl ships as a binary, `rugix-ctrl`, running on your device. This binary is used to query and manage the state of the system, to install updates, and to initiate rollbacks. The state management functionality provided by Rugix Ctrl is optional and you can use it as an update installer only.

Rugix Ctrl does not include any remote _delivery mechanism_ for updates. That is, it implements the mechanism for installing updates without prescribing the way in which updates find their way onto the device. Rugix Ctrl is compatible with several [device management solutions](./advanced/device-management.md) and can also easily be integrated into custom setups, for instance, by including a firmware upload page in your application's UI that hands the update itself off to Rugix Ctrl. We believe that this separation is crucial as it avoids vendor lock-in and gives you the flexibility to integrate updates in the way that makes sense for your device.

In addition to `rugix-ctrl`, which runs on your device, Rugix Ctrl provides a tool `rugix-bundler` to create _update bundles_. Update bundles contain the actual data required to install an update, like filesystems and meta information.

You can download pre-built binaries for various architectures from [the Releases page of Rugix's Git repository](https://github.com/silitics/rugix/releases/).

:::info
The easiest way to use Rugix Ctrl is with [Rugix Bakery](../bakery/index.md), the flexible, user-friendly build system for bespoke Linux distributions developed by the Rugix Project. While being developed together, Rugix Ctrl can also be used completely independently of Rugix Bakery. For commercial customers, Silitics, [the company behind Rugix](/open-source-commitment), offers a [Yocto](https://www.yoctoproject.org/) integration.
:::