---
sidebar_position: 1
---

# Getting Started 🚀

Rugix is a collection of tools designed to build reliable embedded Linux devices with over-the-air update capabilities.
For this guide, you will be using two of these tools: _Rugix Bakery_, a flexible and user-friendly build system for bespoke Linux distributions, and _Rugix Ctrl_, a powerful tool for robust over-the-air system updates and system state management.

With Rugix, our mission is clear: **Simplify the development of embedded Linux devices.**
This quickstart guide will walk you through the steps required to build a production-ready, customized variant of [Debian](https://www.debian.org) with over-the-air update support, which you can readily run on any EFI-compatible system or deploy on a Raspberry Pi.
You will also install an update to your system running in a VM or on a Raspberry Pi.
End-to-end this guide should take less than 30 minutes to complete, even if you have no prior experience with embedded Linux.
So, let's get started and unlock the potential of Rugix for your embedded projects!


## Setup and Installation

First, you need to set up Rugix Bakery.
Rugix Bakery is distributed as a Docker image (for `arm64` and `amd64`).
Thus, to run Rugix Bakery, a working [Docker](https://www.docker.com/) or [Podman](https://podman.io/) installation is required.
On MacOS, please make sure to use the [MacOS virtualization framework and VirtioFS](https://docs.docker.com/desktop/settings/mac/#general), which is the default with recent versions of Docker Desktop.
On Windows, please use [WSL](https://learn.microsoft.com/en-us/windows/wsl/about).

As a convenience, we provide a small shell script named `run-bakery`.
The script runs an ephemeral Docker container with Rugix Bakery and sets everything up as required.
To start a fresh project, create an empty directory and then run:

```shell
curl -O https://raw.githubusercontent.com/silitics/rugix/v0.8.0/bakery/run-bakery && chmod +x ./run-bakery
```

This will download the `run-bakery` shell script from Rugix's GitHub repository and make it executable.
You can then run Rugix Bakery with `./run-bakery`.
If you run `./run-bakery help` you will get usage instructions for Rugix Bakery.

#### Emulation of Foreign Architectures

If you want to build distributions for foreign architectures, you also need to configure [`binfmt_misc`](https://en.wikipedia.org/wiki/Binfmt_misc) for emulation.[^foreign-architecture]
The easiest way to do so, and as we are already using Docker anyway, is by running the following command:

[^foreign-architecture]: A foreign architecture is a CPU architecture that is different from the one of your host machine. For instance, building images for Raspberry Pi (which has a 64-bit ARM CPU) on an x86 Intel or AMD machine requires emulation.

```shell
docker run --privileged --rm tonistiigi/binfmt --install all
```

This will allow you to build distributions for a huge variety of different architectures.


## Initializing the Project

Rugix Bakery ships with a set of project templates to help you get started quickly.
You can list the available templates with:

```shell
./run-bakery init
```

For the purposes of this guide, we are using the `quickstart-guide` template.
To initialize the project with this template run:

```shell
./run-bakery init quickstart-guide
```

You will notice that this command places various files and directories in the current working directory.
The file `rugix-bakery.toml` is the global project configuration file.
It declares build artifacts, such as images for potentially different types of devices.
In addition, there are two directories `recipes` and `layers`.
A _recipe_ describes additions and modifications that should be made to a system being build and a _layer_ combines multiple recipes.
Let's now make some changes and customize our Debian derivate a bit.


## System Customization

The template comes with a recipe `hello-world` which installs a simple static website into the system.
If you like, you can open the file `recipes/hello-world/html/index.html` and make some changes to it.
For instance, you can put your name there.
Once you are done adjusting the page to your liking, open the file `layers/customized.toml`.
As you can see, this file includes several recipes that contribute to the `customized` layer.
In particular, the `hello-world` recipe appears here.
Recipes can have parameters to configure what they do.
To be able to later connect via SSH to the system, place your SSH key in the parameters section:

```toml title="layers/customized.toml"
[parameters."core/ssh"]
root_authorized_keys = """
<INSERT YOUR PUBLIC SSH KEY HERE>
"""
```

Congratulations! You made your first customizations. 🎉

:::info
As you may have noticed by now, Rugix Bakery adopts a declarative approach: You declaratively define the system you want to build by combining recipes to layers and configuring them based on your needs and requirements.
If you are able to write a `Dockerfile` for your application or configure a system with a shell, you should find it easy to write custom recipes for your application and customization needs.
For further details, [check out the documentation on Rugix Bakery](./bakery/).
:::


## Building Images

Now, to bring an actual system to life, you need to build an image.
An image is a binary blob that can be flashed onto a device or used with a VM.
The template specifies multiple images as it gives you the option to run your system on different devices.

Let's say that you would like to build an image for Raspberry Pi 4, you can do that by running:

```shell
./run-bakery bake image customized-pi4
```

This command will build an image `build/images/customized-pi4.img` that you can write to an SD card, e.g., with [Raspberry Pi Imager](https://www.raspberrypi.com/software/).
You can then put this SD card into a Raspberry Pi 4 and boot into the system that you just built.
When visiting the IP address of the Raspberry Pi in your local network in a web browser, you should see the static website with the changes you made.
In addition, you should be able to connect to the Raspberry Pi via SSH as the `root` user.

The images `customized-arm64` and `customized-amd64` can be used with any EFI-compatible 64-bit ARM and x86 system, respectively.
You can write those to an NVMe or USB drive and afterwards boot from the drive on compatible systems.
During the initial boot process, Rugix Ctrl, running on the system, will expand the partition table to match the actual size of the drive.
For VMs, there are also `vm` variants of these images that use a larger image size such that they can be directly used as a VM disk.

Note that all images are based on the shared `customized` layer.
Layers can build on top of each other and the device-specific layers use the `customized` layer as a basis.
This makes it straightforward to build images for different devices.
A typical use case would be to define a base layer with your application and then declare multiple images for different devices, including VMs for testing.


## Running a VM

While you could use the EFI-compatible images to manually create VMs, there is an easier way built directly into Rugix Bakery.
For instance, to run a VM based on the `customized-arm64` image, run the following command:[^new-vm]

[^new-vm]: Running this command will always create a fresh VM. In particular, this means that the SSH host keys of the system do change every time. When connecting via SSH you may first need to remove the old keys from `known_hosts`.

```shell
./run-bakery run customized-arm64
```

You will then see the VM booting on the command line.
When creating the Docker container for Rugix Bakery, the `run-bakery` shell script also sets up port forwarding for SSH.
That means that you can now connect to the running VM with:

```shell
ssh -p 2222 -L 8080:localhost:80 root@127.0.0.1
```

The option `-L 8080:localhost:80` will also forward port `8080` on your machine to the port `80` of the VM.
Hence, you can now also view the static website installed into your system by opening http://127.0.0.1:8080 in your browser.


## Installing an Update

At this point, you have customized the template and know how to built images for different devices with your customizations.
Let's say that you made some further changes to the website and would like to update an existing system, for instance, the VM that you started earlier.
This is where Rugix Ctrl comes into play.
Rugix Ctrl can install updates from system images (in addition to a dedicated format for update bundles).
Hence, as the first step you need to build a compatible image with the changes:

```shell
./run-bakery bake image customized-arm64
```

You can then transfer this image via `scp` to the earlier started VM:

```shell
scp -P 2222 build/images/customized-arm64.img root@127.0.0.1:/root
```

After the upload is complete, the image can be installed as an update with the following command:

```shell
rugpi-ctrl update install /root/customized-arm64.img
```

This will install the image as an update and then reboot the system.
You will be able to observe the reboot from the VM console (the terminal where `./run-bakery run` is running).
Rugix Ctrl's update mechanism typically works by installing the update separately from the previous system (using redundant system partitions).
Hence, while you installed the new version, it is not yet the default and a reboot will revert back to the old version.
This allows you to perform on-device validation, checking that the new version is indeed working as expected in production.
If everything looks good, the update is then committed with:

```shell
rugix-ctrl system commit
```

The update is now complete!

For further details regarding the update process, [check out Rugix Ctrl's documentation](./ctrl/).


## Conclusion

Congratulations on completing the Rugix quickstart guide! 🙌
You have successfully set up Rugix Bakery, customized your Debian-based system, built images for different devices, and learned how to install over-the-air updates with Rugix Ctrl. 
Rugix is designed to simplify the development of embedded Linux devices, making it easier for you to innovate and deploy reliable systems.

While this guide has covered the basics, there's more to learn and explore. We encourage you to dive deeper into both [Rugix Bakery's documentation](./bakery/) and [Rugix Ctrl's documentation](./ctrl/) to discover additional functionalities and best practices.

Happy building! 🚀
