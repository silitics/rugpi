---
sidebar_position: 1
---

# Getting Started 🚀

Rugix is a suite of open-source tools designed to build **reliable** embedded Linux devices with **efficient and secure** over-the-air update capabilities.
For this guide, you will be using two of these tools: _Rugix Bakery_, a flexible and user-friendly build system for bespoke Linux distributions, and _Rugix Ctrl_, a powerful tool for over-the-air system updates and system state management.
While designed to work seamlessly together, you can also **use the Rugix Ctrl without Rugix Bakery** and vice versa.

With Rugix, our mission is clear: **Simplify the development of embedded Linux devices.**
This quickstart guide will take you through the steps required to build a production-ready, customized variant of [Debian](https://www.debian.org) with over-the-air update support, which you can readily run on any EFI-compatible system or deploy on a Raspberry Pi.
You will also install an update to your system running in a VM or on a Raspberry Pi.
End-to-end this guide should take less than 30 minutes to complete, even if you have no prior experience with embedded Linux.
So, let's get started and unlock the potential of Rugix for your embedded projects!


## Setup and Installation

First, you need to set up Rugix Bakery.
Rugix Bakery requires a working [Docker](https://www.docker.com/) or [Podman](https://podman.io/) installation.
On MacOS, please make sure to use the [MacOS virtualization framework and VirtioFS](https://docs.docker.com/desktop/settings/mac/#general), which is the default with recent versions of Docker Desktop.
On Windows, please use [WSL](https://learn.microsoft.com/en-us/windows/wsl/about).
Rugix Bakery is compatible with 64-bit ARM and x86 systems running MacOS or Linux.[^supported-hosts]

[^supported-hosts]: The respective Docker platforms are `linux/arm64` and `linux/amd64`. Rugix Bakery will run on Apple silicon.

Rugix Bakery ships as a small shell script named `run-bakery`.
The script runs an ephemeral Docker container with Rugix Bakery and sets everything up as required.
To start a fresh project, create an empty directory and then run:

```shell
curl -O https://raw.githubusercontent.com/silitics/rugix/v0.8/bakery/run-bakery && chmod +x ./run-bakery
```

This command will download the `run-bakery` shell script from Rugix's GitHub repository and make it executable.
You can then run Rugix Bakery with `./run-bakery`.
If you run `./run-bakery help` you will get usage instructions.

#### Emulation of Foreign Architectures

If you want to build distributions for foreign architectures, you also need to configure [`binfmt_misc`](https://en.wikipedia.org/wiki/Binfmt_misc) for emulation.[^foreign-architecture]
The easiest way to do so, and as we are already using Docker anyway, is by running the following command:

[^foreign-architecture]: A foreign architecture is a CPU architecture that is different from the one of your host machine. For instance, building images for Raspberry Pi (which has a 64-bit ARM CPU) on an x86 Intel or AMD machine requires emulation.

```shell
docker run --privileged --rm tonistiigi/binfmt --install all
```

This will allow you to build Linux distributions for a huge variety of different architectures.


## Initializing the Project

Rugix Bakery comes with a set of project templates to help you get started quickly.
You can list the available templates with:

```shell
./run-bakery init
```

For this guide, we are using the `quickstart-guide` template.
To initialize the project with this template run:

```shell
./run-bakery init quickstart-guide
```

You will notice that this command places various files and directories in the current working directory.
The file `rugix-bakery.toml` is the _project configuration_ file.
It contains a set of _system declarations_ for which you can build artifacts, like system images.
The template specifies multiple systems as it gives you the option to build images for different devices.
In addition, there are two directories `recipes` and `layers`.
A _recipe_ describes additions and modifications that should be made to a system being build and a _layer_ combines multiple recipes.
Let's now make some changes and customize our Debian variant.


## System Customization

The template comes with a recipe `hello-world` which installs a simple static website into the system.
If you like, you can open the file `recipes/hello-world/html/index.html` and make some changes to it.
For instance, you can put your name there.
Once you are done adjusting the page to your liking, open the file `layers/customized.toml`.
As you can see, this file includes several recipes that contribute to the `customized` layer.
In particular, the `hello-world` recipe appears here.
Recipes may have parameters to configure what they do.
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
If you are able to write a `Dockerfile` for your application or configure a system with a shell, you should find it easy to write custom recipes for your application.
For further details, [check out the documentation on Rugix Bakery](./bakery/).
:::


## Building an Image

Now, to bring an actual device to life, you first need to build an image for it.
An image is a binary blob that can be flashed onto a device or used with a VM.
Let's say that you would like to build an image for Raspberry Pi 4, you can do that by running:

```shell
./run-bakery bake image customized-pi4
```

This command will build an image `build/customized-pi4/system.img` that you can directly write to an SD card, e.g., with [Raspberry Pi Imager](https://www.raspberrypi.com/software/).
You can then put this SD card into a Raspberry Pi 4 and boot into the system that you just built.
When visiting the IP address of the Raspberry Pi in your local network in a web browser, you should see the static website with the changes you made.
If you don't know the IP address, try [`http://rugix-template.local`](http://rugix-template.local).
In addition, you should be able to connect to the Raspberry Pi via SSH as the `root` user.
For Raspberry Pi 5, use `customized-pi5` instead of `customized-pi4`.

The system declarations `customized-efi-arm64` and `customized-efi-amd64` can be used to build images for EFI-compatible 64-bit ARM and x86 devices, respectively, which you can directly write to an NVMe or USB drive and then boot from.

:::note
Rugix Bakery will include Rugix Ctrl into the images.
During the initial boot process, Rugix Ctrl, running on the system, will expand the partition table to match the actual size of the SD card, NVMe drive, or USB drive.
:::

Note that all images are based on the shared `customized` layer.
Layers can build on top of each other and the device-specific layers use the `customized` layer as a basis.
This makes it straightforward to build images for multiple devices.
A typical use case would be to define a base layer with your application and then declare multiple systems for different devices and integration testing.


## Running a VM

While you could use the EFI-compatible images to manually create VMs, there is an easier way built directly into Rugix Bakery.
For instance, to run a VM based on the `customized-efi-arm64` system, run the following command:[^new-vm]

[^new-vm]: Running this command will always create a fresh VM. In particular, this means that the SSH host keys of the system do change every time. When connecting via SSH you may first need to remove the old keys from `known_hosts`.

```shell
./run-bakery run customized-efi-arm64
```

If an image for `customized-efi-arm64` has not been built previously, this command will first build an appropriate image, reusing any layers which have already been built previously.
Afterwards, it will start the VM, which you should then see booting.

When creating the Docker container for Rugix Bakery, the `run-bakery` shell script also sets up port forwarding for SSH.
That means that you can now connect to the running VM directly from your terminal with:

```shell
ssh -p 2222 -L 8080:localhost:80 root@127.0.0.1
```

The option `-L 8080:localhost:80` will also forward port `8080` on your machine to the port `80` of the VM.
Hence, you can now view the static website installed into your system by opening http://localhost:8080 in your browser.


## Installing an Update

At this point, you have customized the template and know how to built images for different devices with your customizations.
Let's say that you made some further changes to the website and would like to update an existing system, for instance, the VM that you started earlier.
This is where Rugix Ctrl comes into play.
Rugix Ctrl can install updates from Rugix _update bundles_ (and in some cases system images).
Rugix update bundles are based on a format specifically engineered for efficient and secure over-the-air updates.
The format provides build-in support for cryptographic integrity checks, compression, and adaptive delta updates.

To build an update bundle for the `customized-efi-arm64` system, run:

```shell
./run-bakery bake bundle --without-compression customized-efi-arm64
```

We use `--without-compression` here to not waste our time waiting for the compression.

This command is also going to output a hash for verifying the bundle's integrity.
The hash will have the following form:

```shell
sha512-256:<hex string>
```

Note that this hash is **not** a hash over the entire bundle.
Instead, it is only used to verify a bundle header, which contains further hashes for other parts of the bundle.
This allows Rugix Ctrl to verify parts of the bundle individually and ensure that manipulated data is never written anywhere, even when streaming updates.[^streaming-updates]
For the cryptography nerds, the hash is the root of a [Merkle tree](https://en.wikipedia.org/wiki/Merkle_tree).

[^streaming-updates]: Rugix Ctrl supports streaming updates directly into their final location on a device without intermediate storage. To do so in a secure way requires us to check the integrity of individual “blocks” before writing them. If we would compute a hash over the entire update instead, we would only be able to detect manipulations after processing the entire update.

After building the bundle, you can transfer it via `scp` to the earlier started VM:

```shell
scp -P 2222 build/customized-efi-arm64/system.rugixb root@127.0.0.1:/root
```

When the upload is complete, the bundle can be installed via SSH as an update with the following command:

```shell
rugix-ctrl update install --verify-bundle <hash> /root/system.rugixb
```

Here, `<hash>` is the bundle hash produced by the earlier `bake` command.

This will install the bundle as an update and then reboot the system.
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

While this guide has covered the basics, there's more to learn and explore.
We encourage you to dive deeper into both [Rugix Bakery's](./bakery/) and [Rugix Ctrl's documentation](./ctrl/) to discover additional functionalities and best practices.
In particular, you should read the section on [State Management](./ctrl/state-management.mdx) to understand why any changes that you make to a running system may be lost after a reboot.[^state-management]

[^state-management]: This may be surprising at first, but we consider it a feature that with Rugix the systems you build will typically be immutable and you have to be explicit about the state of the system you want to persist through updates and reboots.

Happy building! 🚀
