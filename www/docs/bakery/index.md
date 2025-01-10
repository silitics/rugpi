# Rugix Bakery

_Rugix Bakery_ is a flexible, user-friendly build system for bespoke Linux distributions. It enables you to build customized variants of binary distributions, such as [Debian](https://debian.org/) and [Alpine Linux](https://alpinelinux.org/), or to build distributions entirely from source, leveraging industry standards like the [Yocto Project](https://www.yoctoproject.org/). While inherently flexible and not tied to any specific distribution, Rugix Bakery ships with ready-to-use integrations for Debian, Alpine Linux, Raspberry Pi OS, and the Yocto Project's Poky reference distribution.

Typically, Rugix Bakery is used to build _full system images_ and Rugix Ctrl _update bundles_ for OTA system updates. System images generally contain a complete Linux root filesystem, a Linux kernel, and other, additional files required for booting a system. For [supported devices](/devices), Rugix Bakery can build bootable images that are ready to be flashed onto a device out of the box. Rugix Bakery allows you to declare multiple system images and other _build artifacts_[^build-artifacts], while partially sharing build configurations and outputs between them. This feature is particularly useful when building different variants of an image for different devices that share a common base.

[^build-artifacts]: For example, individual filesystems, documentation, or a _software bill of materials_ (SBOM).


## Setup and Installation

Rugix Bakery is distributed as a Docker image (for `arm64` and `amd64`), ensuring a reproducible build environment that includes all the required tools and libraries.
Running Rugix Bakery outside of Docker is currently not supported.
So, to build an image locally, a working [Docker](https://www.docker.com/) or [Podman](https://podman.io/) installation is required.
On MacOS, please make sure to use the [MacOS virtualization framework and VirtioFS](https://docs.docker.com/desktop/settings/mac/#general), which is the default with recent versions of Docker Desktop.
For Windows, please use [WSL](https://learn.microsoft.com/en-us/windows/wsl/about).

For convenience, Rugix Bakery ships as a small shell script named `run-bakery`.
The script runs an ephemeral Docker container and sets everything up as required.
To start a fresh Rugix Bakery project, create an empty directory and then run

```shell
curl -O https://raw.githubusercontent.com/silitics/rugix/v0.8.0/bakery/run-bakery && chmod +x ./run-bakery
```

within this directory.
This will download the `run-bakery` shell script from Rugix's GitHub repository and make it executable.
You can then run Rugix Bakery with `./run-bakery`.
If you use a version control system to manage the project, it is good practice to commit the `run-bakery` shell script into the repository so that everyone can run it without any additional setup.

If you run `./run-bakery help` you should get usage instructions for Rugix Bakery. You can use `./run-bakery init` to initialize a Rugix Bakery project for different distributions and devices from one of the provided templates.

The shell script will also pin your project to a specific major version of Rugix Bakery thereby preventing breaking changes from breaking your build. By setting the `RUGIX_BAKERY_IMAGE` environment variable you can also pin Rugix Bakery to a specific Docker image. This adds an additional layer of protection against breaking changes and of security should the Rugix Bakery repository be compromised. For production setups, it is therefore recommended to always pin Rugix Bakery to a specific Docker image.

Rugix Bakery currently requires the container to run in privileged mode such that `chroot` and bind mounts are available within the container. These are required to run tools inside an environment that looks like the final system. In the future, we plan to leverage Linux user namespaces to isolate the build and the system from the host without requiring elevated privileges.[^privileges]

[^privileges]: Existing tools use different approaches to set up an environment that looks like the final system without requiring privileges. For instance, Yocto uses a tool called [Pseudo](https://git.yoctoproject.org/pseudo/about/) (an alternative to the better-known tool [Fakeroot](https://manpages.debian.org/bookworm/pseudo/fakeroot.1.en.html)), to intercepts calls to system APIs via `LD_PRELOAD` and thereby fake a root environment. This approach has limitations, for instance, it does not work with statically-linked binaries and also does not allow starting services binding sockets to ports below 1024. Rugix Bakery strives to provide a container-like environment by using Linux namespaces and process isolation which does not suffer from the same limitations as existing approaches and thereby mimics a real system more closely.


## Build Process: High-Level Introduction

Before we get into further details, let’s first look at Rugix Bakery’s build process from a high-level perspective.

The build process revolves around two key concepts: _layers_ and _recipes_. A _layer_ consists of the _build outputs_ of a specific stage of the build process. Typically, a layer provides a root filesystem and a kernel for a system. Layers can be built on top of each other, thereby reusing and extending an existing root filesystem as well as any other build outputs that are part of the previous layer. In that regard, layers are akin to image layers in Docker.[^yocto-layers] A _recipe_ describes additions and modifications to be made to a layer. A layer is then built by applying the recipes specified in the layer's build configuration, optionally using a _parent layer_ as a base.

[^yocto-layers]: If you are coming from Yocto, Rugix Bakery layers are distinct from Yocto layers in that they contain actual _build outputs_ while Yocto layers add or modify _build metadata and configurations_ used by the build system.


Here is a summary of the core concepts of the build process:

- _Build Outputs_: Files generated during the build process, including intermediate files used by subsequent stages of the build process.
- _Build Artifacts_: Build outputs of particular importance that should be extracted and stored, such as system images.
- _Build Configuration_: A declarative description of the build process and the desired artifacts.
- _Layer_: A collection of build outputs generated by a specific stage of the build process.
- _Recipe_: A reusable, modular description of how to create or modify build outputs.

Rugix Bakery implements a build process that generates build artifacts from layers created by executing recipes.

**Example: Two Device Variants.**
Assume you have an application that you want to integrate into a Debian-based system and then deploy to two different device variants. In Rugix Bakery, you would define a layer for your application building upon a Debian parent layer. The device-specific modifications will then be realized by a layer per device variant using the application layer as a parent. Finally Rugix Bakery will generate a full system image for initial provisioning and a Rugix Ctrl update bundle for each variant, respectively. The figure below shows the corresponding build tree with the final build artifacts at the bottom.

<p>
```mermaid
graph TD;
    root(( ))
    root --> debian-bookworm(debian-bookworm);
    debian-bookworm --> application(application);
    application --> device1(variant1);
    application --> device2(variant2);
    device1 --> image1([image1]);
    device1 --> update1([update1]);
    device2 --> image2([image2]);
    device2 --> update2([update2]);

```
</p>

You can use the same process to build different software variants with a shared base as well. The individual layers will be cached during the build process saving on build time and ensuring that all layers are based on identical parent layers.

:::note
If you are using Rugix Bakery with Yocto, then the entire Yocto build for a given device will typically take place in a device-specific Rugix Bakery root layer. If you do not intend to derive different variants of the same Yocto build, Rugix Bakery may not add much over a pure Yocto setup. At the bare minimum, Rugix Bakery gives you an isolated build environment in terms of the Rugix Bakery Docker image and you can use the other features of Rugix Bakery, e.g., its [integration testing framework](integration-testing.md).
:::

**Building from Source.**
While Rugix Bakery can be used to build applications from source as part of the build process, its primary use case is to assemble pre-built components into a full system. It is completely fine to use Rugix Bakery as a final phase of a larger build pipeline, building binaries and other parts of your application with external tools and then injecting them into a Rugix Bakery build. While building everything from source in a coherent system like Yocto has some advantages, it also introduces a lot of complexity and overhead that may not be worthwhile for every project.[^yocto-reasons] Rugix Bakery's central strength lies in its ability to exploit the stability, familiarity, and the maintenance effort that goes into proven binary distributions like Debian, thereby lowering the entry barrier and required resources to build rock-solid software distributions for embedded and IoT devices, or anywhere a custom Linux OS is needed.

[^yocto-reasons]: Often, reproducibility, license compliance, and fine-grained control of compile-time options are cited as reasons for using Yocto over Debian, however, (1) not only can [most Debian packages be built 100% reproducibly](https://tests.reproducible-builds.org/debian/reproducible.html) but [Debian snapshots](https://snapshot.debian.org/) dating back to 2005 also provide a way to build Debian-based systems fully reproducibly, (2) all Debian packages come with license information that can be used to generate an SBOM and ensure compliance, and (3) the exact sources for all Debian packages are available and can be used to build custom variants of these packages, e.g., with fine-tuned compile-time options. That being said there are many other good reasons for using Yocto, e.g., when you are using a device for which the manufacturer provides a board support package but no support for a binary distribution like Debian. Whether or not the advantages of Yocto are worth the added complexity and required engineering resources should be evaluated based on the requirements of a project.
