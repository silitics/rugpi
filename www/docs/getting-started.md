---
sidebar_position: 1
---

# Getting Started 🚀

Rugpi consists of two components, _Rugpi Bakery_ for building customized system images, and _Rugpi Ctrl_ for maintaining and managing a system.
This quick-start guide takes you through the steps necessary to build a customized system image with Rugpi Bakery.
This image will contain Rugpi Ctrl for managing a system's state and for installing over-the-air system updates.

You can build images locally or with a CI/CD system like GitHub Actions.
Here, we go through the process of building images locally.
For details about running Rugpi Bakery with a CI/CD system, checkout the [user guide's section on CI/CD Integration](./guide/ci-cd-integration).


## Setup and Installation

Rugpi Bakery is distributed as a Docker image (for `arm64` and `amd64`) because it relies on various Linux tools and facilities to build images.
Building images outside of Docker is fundamentally only possible on Linux and not officially supported.
So, to build an image locally, a working [Docker](https://www.docker.com/) or [Podman](https://podman.io/) installation is required.
On MacOS, please make sure to use the [MacOS virtualization framework and VirtioFS](https://docs.docker.com/desktop/settings/mac/#general), which is the default with recent versions of Docker Desktop.
For Windows, please use [WSL](https://learn.microsoft.com/en-us/windows/wsl/about).

For convenience, Rugpi ships a small shell script `run-bakery` for running Rugpi Bakery.
The script runs an ephemeral Docker container and sets everything up as required.
To start a fresh Rugpi project, create an empty directory and then run

```shell
curl -O https://raw.githubusercontent.com/silitics/rugpi/v0.7/bakery/run-bakery && chmod +x ./run-bakery
```

within this directory.
This will download the `run-bakery` shell script from Rugpi's GitHub repository and make it executable.
You can then run Rugpi Bakery with `./run-bakery`.
If you use a version control system to manage the project, it is good practice to commit the `run-bakery` shell script into the repository so that everyone can run it without any additional setup.

If you run `./run-bakery help` you should now get usage instructions for Rugpi Bakery.

### Emulation of Foreign Architectures

If you want to build images for foreign architectures, you also need to configure [`binfmt_misc`](https://en.wikipedia.org/wiki/Binfmt_misc) for emulation.
The easiest way to do so, and as we are already using Docker anyway, is by running the following command:

```shell
docker run --privileged --rm tonistiigi/binfmt --install all
```

This will allow you to build images for a huge variety of different architectures.


## Initializing the Project

To build an image, you first need to create a few configurations files in the project directory.
Rugpi Bakery ships with a set of templates to help you get started quickly.
You can list the available templates by running:

```shell
./run-bakery init
```

To initialize the project with a template, run:

```shell
./run-bakery init <template name>
```

For instance, if you want an image for Raspberry Pi and want it to be based on Raspberry Pi OS, use the `rpi-raspios` template:

```shell
./run-bakery init rpi-raspios
```

Instead, if you want a Debian image bootable on any EFI-compatible system, use the `debian-grub-efi` template:

```shell
./run-bakery init debian-grub-efi
```

Note that a project is not limited to a specific device or family of devices.
By configuring Rugpi Bakery appropriately, you can also build images based on Raspberry Pi OS and Debian all while sharing parts of the build process.
Such setups are, however, beyond the scope of this quick start guide and we refer to the [user guide's section on System Customization](./guide/system-customization) for details.


## Building an Image

After initializing the project with a template, it is time to build your first image.
Most templates will come with multiple images that you can build.
For instance, the `rpi-raspios` template specifies images for different models of Raspberry Pi.

The images are specified in `rugpi-bakery.toml`. You can also list the available images by running:

```shell
./run-bakery list images
```

To build an image, run:

```shell
./run-bakery bake image <image name>
```

For instance, with the `rpi-raspios` template, you can build an image for Raspberry Pi 4 with:

```shell
./run-bakery bake image tryboot-pi4
```

This will build an image `build/images/tryboot-pi4.img`.
This image uses Raspberry Pi's [`tryboot` feature](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html#fail-safe-os-updates-tryboot) for booting and system updates.
It also includes the necessary firmware update for Raspberry Pi 4.
Checkout the comments in `rugpi-bakery.toml` to find an image that is compatible with your specific model of Raspberry Pi.

Congratulations! You built your first image with Rugpi Bakery. 🙌

The resulting images can be written to a storage medium, e.g., an SD card, an NVMe drive, or a thumb drive.
Compatible systems can then directly boot off the storage medium.
On the first boot, Rugpi Ctrl will usually bootstrap the device.
For instance, it will typically automatically repartition the storage medium and create additional file systems.

Feel free to explore the template and modify it according to your needs. 🚀

To learn how to apply your own customizations, checkout the [user guide's section on System Customization](./guide/system-customization).
