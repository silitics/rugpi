---
sidebar_position: 1
---

# Getting Started 🚀

Rugpi consists of two components, _Rugpi Bakery_ for building customized images, and _Rugpi Ctrl_ for maintaining and managing a Rugpi system.
This quick-start guide takes you through the steps necessary to build a custom Rugpi image with Rugpi Bakery.

## Building an Image

You can [build images locally](#building-an-image-locally) or [using a CI system like GitHub Actions](#using-github-actions).

### Using GitHub Actions

By far the fastest and easiest way to get a working image is to use [GitHub Actions](https://github.com/features/actions).
Simply [create a repository](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-repository-from-a-template#creating-a-repository-from-a-template) from our [Rugpi template](https://github.com/silitics/rugpi-template) and GitHub Actions will build the image automatically from your repository.
Any modifications you push to your repository will trigger GitHub Actions to rebuild the image with your customizations.
Please be aware that building an image is a rather resource-heavy process and may quickly consume your CI minutes, if the repository is private.

That was easy! Nevertheless, we recommend reading the next section, so you understand how it works under the hood.

### Building an Image Locally

First, obtain a local copy of the [Rugpi template](https://github.com/silitics/rugpi-template), for instance by [downloading its contents](https://github.com/silitics/rugpi-template/archive/refs/heads/main.zip) or cloning it:

```shell
git clone https://github.com/silitics/rugpi-template
```

Note that Rugpi Bakery is distributed as a Docker image (for `arm64` and `amd64`) because it relies on various Linux tools and facilities to build the image.
Building images outside of Docker is fundamentally only possible on Linux and not officially supported.
So, to build the image locally, a working Docker installation is required.
On MacOS, make sure to use the [MacOS virtualization framework and VirtioFS](https://docs.docker.com/desktop/settings/mac/#general) (the default with recent versions of Docker Desktop).
The template ships with a `run-bakery` shell script (for Linux and MacOS) to run Rugpi Bakery in a temporary container.
For Windows, please run Rugpi Bakery inside WSL.

To print the usage instructions of Rugpi Bakery, in the root directory of the template, run:

```shell
./run-bakery help
```

On a non-`arm64` system, you need to configure [`binfmt_misc`](https://en.wikipedia.org/wiki/Binfmt_misc) to emulate `arm64`.
The easiest way to do so, and as we are already using Docker, is by running the following command:

```shell
docker run --privileged --rm tonistiigi/binfmt --install arm64
```

Building an image is generally achieved by the commend:

```shell
./run-bakery bake image <image name> build/image.img
```

The configuration file `rugpi-bakery.toml` defines the available images.
For instance, to build an image for Raspberry Pi 4 including the necessary firmware update for the `tryboot` boot mechanism, run:

```shell
./run-bakery bake image pi4 build/image-pi4.img
```

The images specified in the template use the `customized` *layer* defined in `layers/customized.toml`.

When you build an image, internally, Rugpi Bakery does the following steps:

1. First, it downloads and extracts a base image of Raspberry Pi OS.
   This is achieved via the following directive:

   ```toml title="layers/customized.toml"
   parent = "core/raspios-bookworm"
   ```

   This will tell Rugpi Bakery to use the layer `raspios-bookworm` provided by Rugpi itself as a basis for the `customized` layer.
   Note that you can define your own base layers.
   They simply contain an URL of the base image to use.

2. Next, the recipes defined in the layer are applied.
   A *recipe* describes modifications to be made to the system.
   For instance, the `core/ssh` recipe enables SSH.
   Recipes can have parameters.
   For instance, the `root_authorized_keys` parameter of the `core/ssh` recipe sets `authorized_keys` for the `root` user.
   To be able to login as `root` via SSH later, you should replace the existing key with your public key.
   In addition to the builtin recipes, you can supply your own recipes.
   In case of the template, the `hello-world` recipe in the `recipes` directory installs a static website which is served by Nginx.
   For further information about recipes, checkout the [user guide's section on System Customization](./guide/system-customization).

3. Finally, after applying all customizations, an image is produced.
   The resulting image is ready to be written to an SD card, e.g., using Raspberry Pi Imager.
   Note that you cannot use Raspberry Pi Imager to apply any configurations like passwords or WiFi settings.
   The template also defines images for other boards than Raspberry Pi 4.
   For further images, we refer to the `rugpi-bakery.toml` configuration file and the [Supported Boards](./guide/supported-boards.md) section of the user guide.

On the first boot, Rugpi Ctrl will repartition the SD card and then boot into the actual system.
Once the system is running, you should be able to visit the static website via the system's IP address and connect via SSH.

Congratulations! You built your first image with Rugpi Bakery. 🙌

Feel free, to change the website in `recipes/hello-world/html` and experiment with the recipes.
As a next step, we recommend reading the [user guide](./guide).
It covers all the details on system customization, state management, and over-the-air updates.
