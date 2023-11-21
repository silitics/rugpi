To build `boot.scr` from `boot.sh`, run:

```sh
mkimage -A arm -O linux -T script -C none -a 0 -e 0 -n "Rugpi Boot Script" -d .boot.sh boot.scr
```

To build `u-boot.bin`, run:

```sh
CROSS_COMPILE=aarch64-linux-gnu- make rpi_3_rugpi_defconfig
CROSS_COMPILE=aarch64-linux-gnu- make -j$(nproc)
```