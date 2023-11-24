---
sidebar_position: 2
---

# Boot Flows

## Tryboot

The `tryboot` boot flow works almost as described in [Raspberry Pi's documentation on the `tryboot` mechanism](https://www.raspberrypi.com/documentation/computers/config_txt.html#example-update-flow-for-ab-booting).
Instead of reading the `tryboot` flag, the hot partition is compared with the default partition to determine whether a commit is necessary.

## U-Boot

On older Raspberry Pi's, without the `tryboot` bootloader feature, we use [U-Boot](https://docs.u-boot.org/en/latest/) as an intermediate bootloader.
Instead of directly loading the Linux kernel, Raspberry Pi's bootloader is instructed to load U-Boot.
U-Boot is then responsible for booting the Linux kernel from the correct partition set.
This works in two stages.
First, U-Boot loads the first stage boot script `boot.scr` from the first partition.
This first stage is responsible for determining the hot partition set (default or spare).
After doing so, it will load the boot environment `boot.env` and second stage boot script `second.scr` from the hot partition set.
After loading these files, the first stage invokes the second stage boot script.
This has the advantage that the second stage script can be updated via the normal update mechanism.
Hence, additional device tree overlays or other boot time configurations can be deployed in a fail-safe way.

For the first stage, two U-Boot environment files are used to determine the hot partition set.
The file `bootpart.default.env` specifies the default partition set.
The file `boot_spare.env` indicates whether to boot from the spare partition.

For the second stage, one U-Boot environment file is used.
The file `boot.env` contains the kernel command line arguments (generated from `cmdline.txt`) stored in the `bootargs` variable and may also override the kernel to boot (`kernel_file`).

We use U-Boot environment files with a CRC32 checksum.
Hence, if files are corrupted, they will not be loaded.

For further details, we refer to the [boot scripts](https://github.com/silitics/rugpi/tree/main/boot/u-boot/scripts).