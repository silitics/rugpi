---
sidebar_position: 2
---

# Boot Flows

## Tryboot

## U-Boot

On older Raspberry Pi's, without the `tryboot` bootloader feature, we use [U-Boot](https://docs.u-boot.org/en/latest/) as an intermediate bootloader.
Instead of directly loading the Linux kernel, Raspberry Pi's bootloader is instructed to load U-Boot.
U-Boot is then responsible for booting the Linux kernel from the correct partition set.

We use three *environment files*:

```text title="bootpart.default.env"
bootpart=2
```

```text title="boot_spare.env"
boot_spare=1
```

```text title="boot.env"
bootargs=console=...
...
```

### On Atomicity of Commits

The file `bootpart.default.env` takes over the role of `autoboot.txt`.
As it is stored on the FAT filesystem of the first partition, it is subject to the same atomicity guarantees.

The file `boot_spare.env` takes over the role of `tryboot`.
It uses a CRC32 checksum.
Should it be corrupted, the default partition will be booted.
