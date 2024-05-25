//! Creates an image.

use std::{
    fs::{self, File},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
};

use anyhow::Context;
use nix::{
    errno::Errno,
    libc::off64_t,
    unistd::{lseek64, Whence},
};
use rugpi_common::{
    boot::{uboot::UBootEnv, BootFlow},
    disk::{
        gpt::gpt_types, mbr::mbr_types, parse_size, DiskId, NumBlocks, Partition, PartitionTable,
        PartitionTableType,
    },
    patch_boot, patch_config,
    utils::units::NumBytes,
    Anyhow,
};
use tempfile::tempdir;
use xscript::{run, Run};

use crate::{
    project::{
        config::{Architecture, IncludeFirmware},
        images::{self, grub_efi_image_layout, pi_image_layout, ImageConfig, ImageLayout},
    },
    utils::prelude::*,
};

pub fn allocate_file(path: &Path, size: u64) -> Anyhow<()> {
    let file = fs::File::create(path)?;
    nix::fcntl::fallocate(
        file.as_raw_fd(),
        nix::fcntl::FallocateFlags::empty(),
        0,
        size as i64,
    )?;
    // run!(["fallocate", "-l", "{size}", path])?;
    Ok(())
}

pub fn copy_sparse(
    src: &mut File,
    dst: &mut File,
    src_offset: u64,
    dst_offset: u64,
    size: u64,
) -> Anyhow<()> {
    let mut src_offset = off64_t::try_from(src_offset).unwrap();
    let dst_offset = off64_t::try_from(dst_offset).unwrap();
    let src_raw_fd = src.as_raw_fd();
    let dst_raw_fd = dst.as_raw_fd();
    lseek64(src_raw_fd, src_offset, Whence::SeekSet)?;
    lseek64(dst_raw_fd, dst_offset, Whence::SeekSet)?;
    let mut total_remaining = usize::try_from(size).unwrap();
    let mut buffer = vec![0; 8192];
    while total_remaining > 0 {
        // If there is no hole, then `next_hole` points to the end of the file as there
        // always is an implicit hole at the end of any file.
        let next_hole = lseek64(src_raw_fd, src_offset, Whence::SeekHole).context("next hole")?;
        lseek64(src.as_raw_fd(), src_offset, Whence::SeekSet).context("seek set")?;
        let chunk_size = usize::try_from(next_hole - src_offset).unwrap();
        let mut chunk_remaining = chunk_size;
        while chunk_remaining > 0 && total_remaining > 0 {
            let chunk_read = buffer.len().min(chunk_remaining).min(total_remaining);
            src.read_exact(&mut buffer[..chunk_read])?;
            dst.write_all(&buffer[..chunk_read])?;
            chunk_remaining -= chunk_read;
            total_remaining -= chunk_read;
        }
        if total_remaining > 0 {
            src_offset = match lseek64(src_raw_fd, next_hole, Whence::SeekData) {
                Ok(src_offset) => src_offset,
                Err(Errno::ENXIO) => {
                    lseek64(
                        dst_raw_fd,
                        total_remaining.try_into().unwrap(),
                        Whence::SeekCur,
                    )?;
                    break;
                }
                error => error.context("seek data")?,
            };
            let hole_size = src_offset - next_hole;
            lseek64(dst_raw_fd, hole_size, Whence::SeekCur)?;
            total_remaining -= usize::try_from(hole_size).unwrap();
        }
    }
    Ok(())
}

pub fn make_image(config: &ImageConfig, src: &Path, image: &Path) -> Anyhow<()> {
    let work_dir = tempdir()?;
    let work_dir = work_dir.path();

    if let Some(parent) = image.parent() {
        fs::create_dir_all(parent).ok();
    }

    // Initialize system root directory from provided TAR file.
    info!("Extracting root filesystem.");
    let root_dir = work_dir.join("system");
    fs::create_dir_all(&root_dir)?;
    run!(["tar", "-xf", src, "-C", &root_dir])?;

    // Create directories for config and boot partitions.
    info!("Creating config and boot directories.");
    let config_dir = work_dir.join("config");
    fs::create_dir_all(&config_dir)?;
    let boot_dir = work_dir.join("boot");
    fs::create_dir_all(&boot_dir)?;

    // Initialize config and boot partitions based the selected on boot flow.
    info!("Initialize boot flow.");
    if let Some(boot_flow) = config.boot_flow {
        match boot_flow {
            BootFlow::Tryboot => {
                initialize_tryboot(&config_dir, &boot_dir, &root_dir)?;
            }
            BootFlow::UBoot => {
                initialize_uboot(config, &config_dir, &boot_dir, &root_dir)?;
            }
            BootFlow::GrubEfi => {
                initialize_grub(config, &config_dir)?;
            }
        }
    }
    // Always copy second stage boot scripts independently of the boot flow.
    if config.boot_flow.is_some() {
        info!("Copy second stage boot scripts.");
        copy(
            "/usr/share/rugpi/boot/u-boot/bin/second.scr",
            boot_dir.join("second.scr"),
        )?;
        copy(
            "/usr/share/rugpi/boot/grub/second.grub.cfg",
            boot_dir.join("second.grub.cfg"),
        )?;
    }

    // Copy firmware to config partition.
    if let Some(include_firmware) = &config.include_firmware {
        info!("Including firmware.");
        match include_firmware {
            IncludeFirmware::Pi4 => include_pi4_firmware(&config_dir)?,
            IncludeFirmware::Pi5 => include_pi5_firmware(&config_dir)?,
        }
    }

    // At this point, everything is initialized and we can compute the partition table.
    let layout = config
        .layout
        .clone()
        .or_else(|| {
            config.boot_flow.map(|boot_flow| match boot_flow {
                BootFlow::Tryboot => pi_image_layout(),
                BootFlow::UBoot => pi_image_layout(),
                BootFlow::GrubEfi => grub_efi_image_layout(),
            })
        })
        .ok_or_else(|| anyhow!("image layout needs to be specified"))?;

    info!("Computing partition table.");
    let table = compute_partition_table(&layout, work_dir).context("computing partition table")?;

    let size_bytes = table.blocks_to_bytes(table.disk_size);

    info!("Allocating image file.");
    if let Some(size) = &config.size {
        let size = parse_size(size)?;
        allocate_file(image, size.into_raw())?
    } else {
        allocate_file(image, size_bytes.into_raw())?;
    }

    info!("Writing image partition table.");
    table.write(image)?;

    if let Some(boot_flow) = &config.boot_flow {
        if matches!(boot_flow, BootFlow::Tryboot | BootFlow::UBoot) {
            let disk_id = match table.disk_id {
                DiskId::Mbr(mbr_id) => mbr_id.into_raw(),
                _ => bail!("unsupported GPT partition layout"),
            };
            info!("Patching boot configuration.");
            patch_boot(&boot_dir, format!("PARTUUID={disk_id:08X}-05"))?;
            info!("Patching `config.txt`.");
            patch_config(boot_dir.join("config.txt"))?;
        }
    }

    // Create filesystems.
    for (layout_partition, image_partition) in layout.partitions.iter().zip(table.partitions.iter())
    {
        let Some(filesystem) = layout_partition.filesystem else {
            continue;
        };
        info!(
            "Creating {} filesystem on partition {} (size: {}).",
            filesystem.as_str(),
            image_partition.number,
            image_partition.size.into_raw()
        );
        match filesystem {
            images::Filesystem::Ext4 => {
                let size = table.blocks_to_bytes(image_partition.size);
                let fs_image = work_dir.join("ext4.img");
                allocate_file(&fs_image, size.into_raw())?;
                if let Some(path) = &layout_partition.root {
                    run!(["mkfs.ext4", "-d", work_dir.join(path), &fs_image])?;
                } else {
                    run!(["mkfs.ext4", &fs_image])?;
                }
                let mut src = File::open(&fs_image)?;
                let mut dst = File::options().write(true).open(&image)?;
                copy_sparse(
                    &mut src,
                    &mut dst,
                    0,
                    table.blocks_to_bytes(image_partition.start).into_raw(),
                    table.blocks_to_bytes(image_partition.size).into_raw(),
                )?;
            }
            images::Filesystem::Fat32 => {
                let size = table.blocks_to_bytes(image_partition.size);
                let fs_image = work_dir.join("fat32.img");
                allocate_file(&fs_image, size.into_raw())?;
                run!(["mkfs.vfat", &fs_image])?;
                if let Some(path) = &layout_partition.root {
                    let fs_path = work_dir.join(path);
                    for entry in fs::read_dir(&fs_path)? {
                        let entry = entry?;
                        run!([
                            "/usr/bin/mcopy",
                            "-i",
                            &fs_image,
                            "-snop",
                            entry.path(),
                            "::"
                        ])?;
                    }
                }
                let mut src = File::open(&fs_image)?;
                let mut dst = File::options().write(true).open(&image)?;
                copy_sparse(
                    &mut src,
                    &mut dst,
                    0,
                    table.blocks_to_bytes(image_partition.start).into_raw(),
                    table.blocks_to_bytes(image_partition.size).into_raw(),
                )?;
            }
        }
    }
    Ok(())
}

fn initialize_tryboot(config_dir: &Path, boot_dir: &Path, root_dir: &Path) -> Anyhow<()> {
    copy(root_dir.join("boot"), &boot_dir)?;
    run!(["rm", "-rf", root_dir.join("boot")])?;
    std::fs::create_dir_all(root_dir.join("boot"))?;
    copy("/usr/share/rugpi/boot/tryboot", &config_dir)?;
    Ok(())
}

fn initialize_uboot(
    config: &ImageConfig,
    config_dir: &Path,
    boot_dir: &Path,
    root_dir: &Path,
) -> Anyhow<()> {
    copy(root_dir.join("boot"), &boot_dir)?;
    run!(["rm", "-rf", root_dir.join("boot")])?;
    std::fs::create_dir_all(root_dir.join("boot"))?;
    copy("/usr/share/rugpi/pi/firmware", &config_dir)?;
    match config.architecture {
        Architecture::Arm64 => {
            copy(
                "/usr/share/rugpi/boot/u-boot/arm64_config.txt",
                config_dir.join("config.txt"),
            )?;
            copy(
                "/usr/share/rugpi/boot/u-boot/bin/u-boot-arm64.bin",
                config_dir.join("u-boot-arm64.bin"),
            )?;
        }
        Architecture::Armhf => {
            copy(
                "/usr/share/rugpi/boot/u-boot/armhf_config.txt",
                config_dir.join("config.txt"),
            )?;
            for model in ["zerow", "pi1", "pi2", "pi3"] {
                copy(
                    format!("/usr/share/rugpi/boot/u-boot/bin/u-boot-armhf-{model}.bin"),
                    config_dir.join(format!("u-boot-armhf-{model}.bin")),
                )?;
            }
        }
        _ => {
            bail!(
                "no U-Boot support for architecture `{}`",
                config.architecture.as_str()
            );
        }
    }
    copy(
        "/usr/share/rugpi/boot/u-boot/bin/boot.scr",
        config_dir.join("boot.scr"),
    )?;
    std::fs::write(config_dir.join("cmdline.txt"), "")?;

    let mut env = UBootEnv::new();
    env.set("bootpart", "2");
    env.save(config_dir.join("bootpart.default.env"))?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "0");
    env.save(config_dir.join("boot_spare.disabled.env"))?;
    env.save(config_dir.join("boot_spare.env"))?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "1");
    env.save(config_dir.join("boot_spare.enabled.env"))?;

    Ok(())
}

fn initialize_grub(config: &ImageConfig, config_dir: &Path) -> Anyhow<()> {
    std::fs::create_dir_all(config_dir.join("EFI/BOOT")).ok();
    copy(
        "/usr/share/rugpi/boot/grub/first.grub.cfg",
        config_dir.join("EFI/BOOT/grub.cfg"),
    )?;
    std::fs::create_dir_all(config_dir.join("rugpi")).ok();
    copy(
        "/usr/share/rugpi/boot/grub/bootpart.default.grubenv",
        config_dir.join("rugpi/bootpart.default.grubenv"),
    )?;
    copy(
        "/usr/share/rugpi/boot/grub/boot_spare.grubenv",
        config_dir.join("rugpi/boot_spare.grubenv"),
    )?;
    match config.architecture {
        Architecture::Arm64 => {
            copy(
                "/usr/lib/grub/arm64-efi/monolithic/grubaa64.efi",
                config_dir.join("EFI/BOOT/BOOTAA64.efi"),
            )?;
        }
        Architecture::Amd64 => {
            copy(
                "/usr/lib/grub/x86_64-efi/monolithic/grubx64.efi",
                config_dir.join("EFI/BOOT/BOOTX64.efi"),
            )?;
        }
        _ => {
            bail!(
                "no Grub support for architecture `{}`",
                config.architecture.as_str()
            );
        }
    }
    Ok(())
}

fn include_pi4_firmware(config_dir: &Path) -> Anyhow<()> {
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2711/stable/pieeprom-2023-05-11.bin",
        config_dir.join("pieeprom.upd")
    ])?;
    run!([
        "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
        "-i",
        config_dir.join("pieeprom.upd"),
        "-o",
        config_dir.join("pieeprom.sig")
    ])?;
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2711/stable/vl805-000138c0.bin",
        config_dir.join("vl805.bin")
    ])?;
    run!([
        "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
        "-i",
        config_dir.join("vl805.bin"),
        "-o",
        config_dir.join("vl805.sig")
    ])?;
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2711/stable/recovery.bin",
        config_dir.join("recovery.bin")
    ])?;
    Ok(())
}

fn include_pi5_firmware(config_dir: &Path) -> Anyhow<()> {
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2712/stable/pieeprom-2023-10-30.bin",
        config_dir.join("pieeprom.upd")
    ])?;
    run!([
        "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
        "-i",
        config_dir.join("pieeprom.upd"),
        "-o",
        config_dir.join("pieeprom.sig")
    ])?;
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2712/stable/recovery.bin",
        config_dir.join("recovery.bin")
    ])?;
    Ok(())
}

fn copy(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Anyhow<()> {
    let dst = dst.as_ref();
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).ok();
    };
    run!(["cp", "-rTp", src.as_ref(), dst])?;
    Ok(())
}

/// We are calculating everything with a portable block size of 512 bytes.
const BLOCK_SIZE: NumBytes = NumBytes::from_raw(512);

/// We align everything to 2048 blocks, i.e., 1MiB.
const ALIGNMENT: NumBlocks = NumBlocks::from_raw(2048);

/// Convert number of bytes to number of blocks.
fn bytes_to_blocks(bytes: NumBytes) -> NumBlocks {
    NumBlocks::from_raw(bytes.into_raw().div_ceil(BLOCK_SIZE.into_raw()))
}

/// Compute the partition table for an image based on the provided layout.
fn compute_partition_table(layout: &ImageLayout, work_dir: &Path) -> Anyhow<PartitionTable> {
    let table_type = layout.ty;
    let mut partitions = Vec::new();
    let mut next_usable = ALIGNMENT;
    let mut next_number = 1;
    let mut in_extended = false;
    for partition in &layout.partitions {
        // Partitions are numbered based on their appearance in the layout.
        let number = next_number;
        next_number += 1;
        if table_type.is_mbr() && number > 4 && !in_extended {
            bail!("invalid number of primary partitions in MBR");
        }
        // Leave space for the EBR, if we are creating a logical MBR partition.
        if in_extended {
            next_usable = (next_usable + NumBlocks::ONE).ceil_align_to(ALIGNMENT);
        }
        // By default, we create `LINUX` partitions.
        let partition_type = partition.ty.unwrap_or(match table_type {
            PartitionTableType::Mbr => mbr_types::LINUX,
            PartitionTableType::Gpt => gpt_types::LINUX,
        });
        if partition_type.table_type() != layout.ty {
            bail!("partition type `{partition_type}` does not match table type `{table_type}`",)
        }
        // The start of the partition is the next usable block.
        let start = next_usable;
        if partition_type.is_extended() {
            if in_extended {
                bail!("nested extended partitions are not allowed")
            }
            partitions.push(Partition {
                number,
                start,
                // We fix this later once we know the size of the extended part.
                size: 0.into(),
                ty: partition_type,
                name: partition.label.clone(),
                gpt_id: None,
            });
            in_extended = true;
            next_number = 5;
            // Space for the EBR is automatically added prior to the next partition.
        } else {
            let size = match &partition.size {
                Some(size) => bytes_to_blocks(parse_size(size)?),
                None => {
                    let Some(path) = &partition.root else {
                        bail!("partitions without a fixed size must have a root path");
                    };
                    compute_fs_size(work_dir.join(path))?
                }
            };
            partitions.push(Partition {
                number,
                start,
                size,
                ty: partition_type,
                name: partition.label.clone(),
                gpt_id: None,
            });
            next_usable = (start + size).ceil_align_to(ALIGNMENT);
        }
    }
    // Fix the size of the extended partition, if there is one.
    for partition in partitions.iter_mut() {
        if !partition.ty.is_extended() {
            continue;
        }
        partition.size = (next_usable - partition.start + NumBlocks::ONE).ceil_align_to(ALIGNMENT);
        break;
    }
    // Create and validate the partition table.
    let image_size = match partitions.last() {
        Some(last_partition) => {
            (last_partition.start + last_partition.size).ceil_align_to(ALIGNMENT) + ALIGNMENT
        }
        None => ALIGNMENT * 32,
    };
    let table_id = match table_type {
        PartitionTableType::Mbr => DiskId::random_mbr(),
        PartitionTableType::Gpt => DiskId::random_gpt(),
    };
    let mut table = PartitionTable::new(table_id, image_size);
    table.partitions = partitions;
    table.validate()?;
    Ok(table)
}

/// Compute the required size for a filesystem based on the given root path.
fn compute_fs_size(root: PathBuf) -> Anyhow<NumBlocks> {
    let mut size = NumBytes::from_raw(0);
    let mut stack = vec![root];
    while let Some(top) = stack.pop() {
        // We do not want to follow symlinks here as we are interested in the size of
        // the symlink and no the size of the symlink's target.
        let metadata = fs::symlink_metadata(&top)?;
        size += NumBytes::from_raw(metadata.size());
        if metadata.is_dir() {
            for entry in fs::read_dir(&top)? {
                stack.push(entry?.path());
            }
        }
    }
    // Add an overhead of 10% for filesystem metadata.
    size += NumBytes::from_raw(size.into_raw().div_ceil(10));
    Ok(bytes_to_blocks(size))
}
