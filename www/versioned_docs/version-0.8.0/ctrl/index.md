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
Therefore, an update solution must provide mechanisms to prevent manipulated updates from being installed.

Rugix Ctrl addresses these challenges by ensuring atomic updates, on-device validation with rollback capabilities, reliable state management, and protection against malicious updates.
By utilizing Rugix Ctrl, you can rest assured that your devices remain reliable, secure, and up-to-date, **allowing you to focus on delivering value to your users**.


## High-Level Overview

Rugix Ctrl is designed around _full system updates_.
That is, instead of updating individual parts of your system, such as individual libraries or just your application, Rugix Ctrl will typically update the system as a whole.[^incremental-updates]
Full system updates are advantageous because they allow you to test all components together and ensure a consistent environment across devices.
If full system updates sound expensive in terms of download size, don't worry, Rugix has support for _delta updates_, adaptively reducing the download size to the parts of the system that actually changed.
This gives you the advantages of full system updates at almost no extra cost.

[^incremental-updates]: Rugix Ctrl also supports incremental updates.

Rugix Ctrl is an update installer and does not include any remote delivery mechanism for updates.
That is, it implements a mechanism for installing updates without prescribing the way in which updates find their way onto the device.
We believe that this separation is crucial as it avoids vendor lock-in and gives you the flexibility to integrate updates in the way that makes sense for your device.
To manage your devices and push updates to them, Rugix Ctrl is compatible with several [fleet management solutions](./advanced/fleet-management.md) and can be easily integrated into your application, e.g., by offering your users the ability to upload a firmware update in your own UI.

Rugix Ctrl ships as a binary, `rugix-ctrl`, running on your device.
This binary is used to query and manage the state of the system, to install updates, and to initiate rollbacks.
The state management functionality provided by Rugix Ctrl is completely optional and you can use Rugix Ctrl as an update installer only, if you wish.
In addition to `rugix-ctrl`, which runs on your device, Rugix Ctrl also provides a tool, `rugix-bundler`, to create _update bundles_.
Update bundles contain the actual data required to install an update, like filesystems and some meta information.
You find pre-built binaries of these tools on [the Releases page of Rugix's Git repository](https://github.com/silitics/rugix/releases/).

:::tip
The easiest way to use Rugix Ctrl is with [Rugix Bakery](../bakery/index.md), a flexible and user-friendly build system for bespoke Linux distributions developed by the Rugix Project.
With Rugix Bakery, it is straightforward to integrate Rugix Ctrl into your system.
Furthermore, Rugix Bakery also includes `rugix-bundler` and can directly create update bundles for Rugix Ctrl.
:::

The following documentation focuses on the concepts behind Rugix Ctrl and its usage.
For the most part, it will assume that you already have a working integration of Rugix Ctrl into your system, which you get out-of-the-box when you use Rugix Bakery to build the system.
Among other things, the section [Advanced Topics](./advanced/) of this documentation covers how Rugix Ctrl can be integrated into and adapted for other build systems and setups.
Note that while being developed together, Rugix Ctrl can also be used completely independently of Rugix Bakery.
For commercial customers, Silitics, [the company behind Rugix](/open-source-commitment), offers a [Yocto](https://www.yoctoproject.org/) integration.

## Comparison to Other Solutions

:::info
This section is meant for those already familiar with other over-the-air update solutions as a quick way to decide whether Rugix Ctrl is worth a closer look.
It goes into specific technical details and features.
If you are new to over-the-air updates, be assured that Rugix Ctrl strives to and does serve almost any use case, so you may [just skip this section](./over-the-air-updates.mdx).
:::

We believe that Rugix Ctrl is a good choice for almost any use case where over-the-air updates are required.
However, a fair comparison between the different solutions in the space is challenging as the various tools adopt vastly different approaches and cutting the space of functionality into distinct feature categories will always be subjective to a degree.
Furthermore, most tools have built-in support for user-defined functionality through which their features can be extended.
Nevertheless, here is our attempt to compare the tools.

For our comparison, we consider the following solutions in addition to Rugix Ctrl.

[*Mender*](https://docs.mender.io/artifact-creation/standalone-deployment) is an open-source over-the-air (OTA) software updater for embedded Linux devices and a fleet management solution.

[*RAUC*](https://rauc.io/) (Robust Auto-Update Controller) is a lightweight and flexible update solution designed for embedded systems. It supports various update scenarios and provides robust mechanisms to ensure the integrity and reliability of updates.

[*SWUpdate*](https://sbabic.github.io/swupdate/swupdate.html) (Software Update) considers itself an update framework for embedded systems. It provides foundational building blocks that can be flexibly combined to build tailored update workflows for different scenarios and use cases.

Let's start with the uncontroversial facts about licenses and programming languages:[^prog-lang]

[^prog-lang]: The programming language may be relevant, if you want/need to extend the solution yourself.

| | Mender | RAUC | SWUpdate | Rugix Ctrl |
| -: | :-: | :-: | :-: | :-: |
| License | Apache-2.0 | LGPL-2.1 | GPL-2.0 | MIT/Apache-2.0 |
| Language | C++ | C | C | Rust |

All the solutions we consider here are open-source and can be used in commercial products.

Being written in Rust, a [memory-safe language](https://en.wikipedia.org/wiki/Memory_safety), Rugix Ctrl has a reduced surface for any [memory-related security vulnerabilities](https://en.wikipedia.org/wiki/Memory_safety#Impact).
We take this to be an advantage over all the other solutions as updates are an inherently security-sensitive issue.

#### General Remarks

Before we get into specific features, a few more general remarks about the different solutions are in order.

Mender is a full fleet management solution whose update client can be used to install updates without adopting the fleet management solution itself.
In contrast, all the other solutions are standalone update solutions.
When you use Mender for over-the-air updates, you will find that it has been designed for usage with the fleet management solution.
Therefore, it is generally less flexible than the other solutions which can lead to challenges if your use case does not align well with its rigidity.

SWUpdate considers itself a framework and provides a lot of flexibility to build your own update workflows.
This, however, also means that you need to invest the necessary time to flesh out all the details.
In contrast, RAUC and Rugix Ctrl are more opinionated in how you should structure your update process, while still providing enough flexibility for almost all use cases.
If you don't want to become an expert in the low-level details of updates, then a more opinionated solution may be the better choice.

Rugix Ctrl provides a unique (but optional) approach to state management inspired by container-based architectures.
While all solutions provide state management facilities, Rugix Ctrl's approach makes it straightforward to selectively persist system state through updates, protects against accidental state and system partition corruption, and offers off-the-shelf factory reset functionality.
If it does not suit your needs, you can also opt-out of the state management mechanism and instead use a more traditional approach to state management comparable to what all the other solutions considered here offer.

#### Feature-Wise Comparison

Now, here is the promised feature-wise comparison of the different solutions.[^contribute-comparison]

[^contribute-comparison]: If you think that this comparison is unfair, inaccurate, or lacks certain important features, please [open an issue](https://github.com/silitics/rugix/issues/new/choose).

| | Mender | RAUC | SWUpdate | Rugix Ctrl | Description |
| - | :-: | :-: | :-: | :-: | - |
| Streaming: Arbitrary Sources | ❌ | ❌ | ✅ | ✅ | Streaming updates from arbitrary sources. |
| Streaming: HTTP | ✅ | ✅ | ✅ | ✅ | Streaming updates from an HTTP server. |
| Delta Updates: Adaptive | ❌ | ✅ | ✔️[^build-yourself] | ✅ | Fetch only changed blocks via HTTP. |
| Delta Updates: Static | ✔️[^mender-delta] | ❌ | ✔️[^build-yourself] | ✔️[^build-yourself] | Offline delta compression. |
| Non-A/B Update Schemes | ❌[^mender-update-modules] | ✅ | ✅ | ✅ | Support for non-A/B rootfs updates.
| Update Scripts | ✅ | ✅ | ✅ | ✅ | Ship and run scripts as part of an update. |
| Arbitrary Update Payloads | ✅ | ✅ | ✅ | ✅ | Support for arbitrary update payloads.
| Bootloaders: Grub | ✅ | ✅ | ✅ | ✅ | Support for Grub. |
| Bootloaders: U-Boot | ✅ | ✅ | ✅ | ✅ | Support for U-Boot. |
| Bootloaders: Barebox | ❌ | ✅ | ❌ | ❌ | Support for Barebox. |
| Bootloaders: Tryboot[^tryboot] | ❌ | ❌ | ❌ | ✅ | Support for Tryboot. |
| Bootloaders: Custom | ❌ | ✅ | ✅ | ✅ | Custom bootloader integrations. |
| Security: Artifact Verification | ✅  | ✅ | ✅  | ✅ | Check the integrity of the update as a whole. |
| Security: Block-Wise Verification | ❌ | ✅ | ❌ | ✅ | Check blocks individually before writing them. |
| Security: Embedded Signatures | ✅ | ✅ | ✅ | ❌ | Embed signatures into an update. |
| Security: External Signatures | ❌ | ❌ | ❌ | ✅ | Use an external signature/root of trust. |
| Security: Encrypted Updates | ❌ | ✅ | ✅ | ❌ | Encrypted update artifacts.
| Yocto Integration | ✅ | ✅ | ❌[^swu-yocto] | ✔️[^rugix-yocto] | Ready-made Yocto integration. |

[^mender-update-modules]: With Mender's update modules you could build this yourself, however, there is no built-in support.
[^mender-delta]: Only supported in the enterprise version, not the open-source version.
[^build-yourself]: You can build this yourself using third-party tools.
[^tryboot]: Official mechanism to realize A/B updates on Raspberry Pi.
[^swu-yocto]: You need to build this yourself based on your concrete update workflow.
[^rugix-yocto]: Available commercially from [Silitics](/commercial-support).
