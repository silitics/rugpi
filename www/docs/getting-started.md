---
sidebar_position: 1
---

# Getting Started 🚀

Rugpi consists of two components, _Rugpi Bakery_ for building customized images, and _Rugpi Ctrl_ for maintaining and managing a Rugpi system.
This quick-start guide takes you through the steps necessary to build a custom Rugpi image with Rugpi Bakery.

⚠️ This quick-start guide assumes that you are building an image for Raspberry Pi 4.
While the workflow is the same for other models, they may need different settings.
For other models, please read the [Supported Boards](./guide/supported-boards.md) section of the user guide.

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
The template ships with a `run-bakery` shell script (for Linux and macOS) to run Rugpi Bakery in a temporary container.
For Windows, an equivalent `run-bakery.bat` batch file is provided.

To print the usage instructions of Rugpi Bakery, in the root directory of the template, run:

```shell
./run-bakery help
```

On a non-`arm64` system, you need to configure [`binfmt_misc`](https://en.wikipedia.org/wiki/Binfmt_misc) to emulate `arm64`.
The easiest way to do so, and as we are already using Docker, is by running the following command:

```shell
docker run --privileged --rm tonistiigi/binfmt --install arm64
```

Building an image is generally a three-step process:

1. First, you must extract all system files from a base image of Raspberry Pi OS.
   To this end, the `extract` command is used:

   ```shell
   ./run-bakery extract <path to base image> build/base.tar
   ```

   The path to the base image can also be a URL.
   So, for instance, to use Raspberry Pi OS Lite as the basis for your image, run:

   ```shell
   ./run-bakery extract https://downloads.raspberrypi.org/raspios_lite_arm64/images/raspios_lite_arm64-2023-05-03/2023-05-03-raspios-bullseye-arm64-lite.img.xz build/base.tar
   ```

   This command produces a `.tar` archive `build/base.tar` in the `build` directory of the template.

2. Next, you apply your customizations.
   The template contains a `rugpi-bakery.toml` configuration file for Rugpi Bakery.
   The template's configuration will enable a few *recipes* with the `recipes` directive.
   A *recipe* describes modifications to be made to the system.
   For instance, the `ssh` recipe enables SSH.
   Recipes can have parameters.
   For instance, the `root_authorized_keys` parameter of the `ssh` recipe sets `authorized_keys` for the `root` user.
   To be able to login as `root` via SSH later, you should replace the existing key with your public key.
   In addition to the builtin recipes, you can supply your own recipes.
   In case of the template, the `hello-world` recipe in the `recipes` directory installs a static website which is served by Nginx.
   For further information about recipes, checkout the [user guide's section on System Customization](./guide/system-customization).

   To customize the base system, according to the `rugpi-bakery.toml`, and with all recipes in the `recipes` directory, run:

   ```shell
   ./run-bakery customize build/base.tar build/customized.tar
   ```

   This command produces a `.tar` archive `build/customized.tar` with the customized system.

3. Finally, after applying all customizations, an image is produced with:

   ```shell
   ./run-bakery bake build/customized.tar build/customized.img
   ```

   The resulting image `build/image.img` is now ready to be written to an SD card, e.g., using Raspberry Pi Imager.

   On the first boot, Rugpi Ctrl will repartition the SD card and then boot into the actual system.
   Once the system is running, you should be able to visit the static website via the system's IP address and connect via SSH.

Congratulations! You built your first image with Rugpi Bakery. 🙌

Feel free, to change the website in `recipes/hello-world/html` and experiment with the recipes.
As a next step, we recommend reading the [user guide](./guide).
It covers all the details on system customization, state management, and over-the-air updates.
