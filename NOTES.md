# Bootloaders

Commands for installing Grub:

```shell
grub-install \
    --target i386-pc \
    --boot-directory boot /dev/loop0

EFI_TARGET="x86_64-efi"
grub-install \
    --target "${EFI_TARGET}" \
    --efi-directory boot \
    --boot-directory boot \
    --no-nvram \
    --bootloader-id rugpi \
    --removable \
    /dev/loop0 
```

To install Systemd Boot, we copy the EFI files manually:

```shell
cp /usr/lib/systemd/boot/efi/systemd-bootx64.efi boot/EFI/BOOT/BOOTX64.efi
cp /usr/lib/systemd/boot/efi/systemd-bootaa64.efi boot/EFI/BOOT/BOOTAA64.efi
cp /usr/lib/systemd/boot/efi/systemd-bootarm.efi boot/EFI/BOOT/BOOTARM.efi
```

# Verity Streams

At some point, we may want to support checking of streamed updates to make sure that only verified data is written to disk.

Ensuring the integrity of update artifacts is paramount.

When streaming an artifact to Rugpi, we may want to ensure that it has not been tempered with.
To this end, Rugpi supports *verity streams*.
The goal is that we do not want Rugpi to write anything to a partition or some other place that has not been verified *prior* to writing it.

The verity stream can then contain an image or a Rugpi bundle.

```
rugpi update install --verity-hash sha256:uU0nuZNNPgilLlLX2n2r-sSE7-N6U4DukIj3rOLvzek ...
```

A verity stream is a linked list of blocks.
The provided hash is used to verify the header, containing the hash of the first block.
The first block then contains the hash of the next block and so.
In addition, the header contains the size and other information such that we can make sure that data is not truncated and that we can efficiently decode a stream if we do not care about hashing at all.

Binary format:

```
HEADER:
MAGIC: [u8; 16]
VERSION: u16
ALGORITHM: u16
SIZE: u64
BLOCK_SIZE: u32
HASH_SIZE: u16
FIRST_HASH: u8[HASH_SIZE]

BLOCK:
NEXT_HASH: u8[HASH_SIZE]
DATA: u8[BLOCK_SIZE]
```

Some commands that we may want to build:

```
rugpi-verity create <input> <output>
```

```
rugpi-verity verify <hash>

cat <output> | rugpi-verity verify <hash> >verified.data
```

# System and Artifact Layouts

Currently Rugpi simply chooses an appropriate layout.
Under the hood, this is implemented as a more flexible mechanism, parts of which we may want to expose to users at some point.

A *partition schema* defines how the partitions of a system should be layed out.

A *partition* identifies a space on a block device.

A *slot* is a set of *slot entries*.
A *slot entry* is a partition and an optional path.

1. Map the slots of the artifact to the slots of the system.
2. Ensure that the slots of the system are *cold*.
3. For each of the slot pairs: Install the slot entry from the artifact to the slot entry of the system.

Slots can have overlapping partitions.

Partitions can be redundant, in which case, they consist of multiple copies one after the other.

Example slots for Systemd Boot:

System Slots:
```
a:
    entry: EFI/loader/entries/rugpi-a.conf
    boot: EFI/rugpi/a
    system: system-a
b:
    entry: EFI/loader/entries/rugpi-b.conf
    boot: EFI/rugpi/b
    system: system-b
```

Image Slots:
```
image:
    entry: EFI/loader/entries/rugpi-a.conf
    boot: EFI/rugpi/a
    system: system-a
```

Example slots for Tryboot:

System Slots:
```
a:
    boot: boot-a
    system: system-a
b:
    boot: boot-b
    system: system-b
```


Image Slots:
```
image:
    boot: boot-a
    system: system-a
```