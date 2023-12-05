---
slug: first-release
title: Version 0.5
authors: koehlma
tags: [rugpi,release]
unlisted: true
---

We are excited to announce the release of Rugpi version 0.5. 🎉

Version 0.5 signifies the end of Rugpi's experimental phase.
We are confident that the foundational update mechanism is sound.
From this point forward, we aim to **maintain backwards-compatibility for updates**.
This means, if you deploy a system with Rugpi now, you should be able to update it remotely later.
While the update process itself is stable, we are still iterating on the design of the image building pipeline and the CLI and APIs.
What will change in the upcoming months is the way system images are build.
We are planning to introduce *layers*, drawing inspiration from Docker.
Layers will streamline the image-building process and enable fail-safe delta updates in the future.

### What's new?

On top of conducting further testing of the build and update process, this release also brings valuable new features over the initial prototype, which [we introduced back in July](2023-07-23-introducing-rugpi.md).

Rugpi now **supports all models of Raspberry Pi** and 32-bit Raspberry Pi OS.
We have tested the images on various models.
A big shout out and many thanks go to [Reuben Miller](https://github.com/reubenmiller), who helped significantly with testing.
In particular, Rugpi is known to work on Raspberry Pi Zero 2 W, a low-cost but still powerful variant of Raspberry Pi.
Having extended support beyond the latest boards by integrating with [U-Boot](https://docs.u-boot.org/en/latest/), we are now also confident that Rugpi can be brought to other boards than Raspberry Pi.
While building an amazing, optimized experience for Raspberry Pi clearly remains our focus, feel free to contact us in case you like Rugpi and want to use it with a different single-board computer.

In addition to support for all models of Raspberry Pi, Rugpi now also supports **streaming updates directly to the underlying storage**.
Furthermore, there is now an option to persist the writeable overlay by default making it easier to persist user-defined customizations in the field.

### What's next?

Having validated and expanded the foundational design, we now look ahead to the future.
We plan to introduce a layer system to Rugpi.
Layers will be cached and shared between different variants of a system image.
Furthermore, they will enable a crucial feature, **fail-safe delta updates**.

### Industry Adoption and Collaboration

We saw great positive resonance to our initial prototype.
With [thin-edge.io](https://thin-edge.io) we are proud to have found a partner spearheading the way to industry adoption.
Thin-edge.io officially supports Rugpi to build and deploy images.
To learn more, checkout the [thin-edge.io Rugpi reference repository](https://github.com/thin-edge/tedge-rugpi-image).

If you're planning to adopt Rugpi or have feedback to share, we want to hear from you!
Your contributions and insights are invaluable as we continue to shape the future of Rugpi.
Join the community and share your experiences with Rugpi [by opening a discussion on GitHub](https://github.com/silitics/rugpi/discussions).