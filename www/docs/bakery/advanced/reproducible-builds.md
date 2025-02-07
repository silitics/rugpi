---
sidebar_position: 5
---

# Reproducible Builds

At Rugix, we are huge proponents of [*reproducible builds*](https://reproducible-builds.org/).

> “A build is *reproducible* if given the same source code, build environment and build instructions, any party can recreate bit-by-bit identical copies of all specified artifacts.” – https://reproducible-builds.org/docs/definition/

We aim to enable reproducible builds of Rugix itself as well as of system images built with Rugix.

**Note that this is a work-in-progress effort and we are not there yet.**


## Reproducibility of Rugix

> Given a commit hash of Rugix, everyone should be able to reproduce all Rugix-related binaries that may end up in images as well as the Docker image for Rugix Bakery. This includes binaries of third-party software, e.g., Grub and U-Boot.

Reproducible builds require a known build environment.
To this end, we use [Debian's official snapshots](https://snapshot.debian.org/) as a basis.
Note that most [Debian packages are also fully reproducible](https://tests.reproducible-builds.org/debian/reproducible.html), so one could in principle trace everything back its source code.
Building upon Debian snapshots also has the advantage that it reduces the surface area that must be checked for Rugix, provided that one trusts Debian.

To obtain a reproducible build environment, we follow a two stage process:

- Stage 0: We build a Debian Docker image containing [`mmdebstrap`](https://manpages.debian.org/bookworm/mmdebstrap/mmdebstrap.1.en.html).
This image will not be reproducible as it is based on the latest version of Debian Bookworm.
We are only using it to bootstrap Stage 1.
- Stage 1: Using the previously built Docker image, we bootstrap a Debian Docker image based on an official snapshot.
This image will be fully reproducible and everything we install will be pinned to the respective snapshot.
Thereby, all images that we derive from it, e.g., to build Rugix, Grub, or U-Boot, are also fully reproducible.

Everything else we built will be based on the reproducible Stage 1 image.

The infrastructure for reproducible builds is implemented as part of Rugix's [`xtask`](https://github.com/silitics/Rugix/tree/main/xtask).


## Reproducibility of Images

**🚧 This is work-in-progress. 🚧**

Reproducibility of Rugix is a prerequisite for reproducible system images.

Furthermore, reproducibility of images requires support by the underlying Linux distribution.
In particular, it requires that a distribution provides immutable snapshots of their repositories (rebuilding packages is outside the scope of Rugix).

Among the distributions supported by Rugix, only Debian [officially provides immutable snapshots](https://snapshot.debian.org/) of their repositories.
Furthermore, Debian also participates in the Reproducible Builds project's [continuous reproducibility testing of their packages](https://tests.reproducible-builds.org/debian/reproducible.html).
We aim to eventually support building fully reproducible Debian images based on the official snapshots.

For other distributions and base layers built with third-party tools (e.g., Yocto Project and Buildroot), we aim to make images reproducible under the assumption that the external inputs (repositories, base layer, …) do not change.