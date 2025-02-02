use byte_calc::NumBytes;

use rugix_common::disk::gpt::gpt_types;
use rugix_common::disk::mbr::mbr_types;

use crate::config::images::{Filesystem, ImageLayout, ImagePartition, PartitionTableType};
use crate::config::systems::Target;

pub mod generic_grub_efi;
pub mod rpi_tryboot;
pub mod rpi_uboot;

/// Get the default image layout for the provided target.
pub fn get_default_layout(target: &Target) -> Option<ImageLayout> {
    match target {
        Target::GenericGrubEfi => Some(default_gpt_layout()),
        Target::RpiTryboot => Some(default_mbr_layout()),
        Target::RpiUboot => Some(default_mbr_layout()),
        Target::Unknown => None,
    }
}

fn default_mbr_layout() -> ImageLayout {
    ImageLayout::new()
        .with_ty(Some(PartitionTableType::Mbr))
        .with_partitions(Some(vec![
            // Config partition.
            ImagePartition::new()
                .with_size(Some(NumBytes::mebibytes(256)))
                .with_ty(Some(mbr_types::FAT32_LBA))
                .with_filesystem(Some(Filesystem::Fat32))
                .with_root(Some("config".to_owned())),
            // `A` boot partition.
            ImagePartition::new()
                .with_size(Some(NumBytes::mebibytes(128)))
                .with_ty(Some(mbr_types::FAT32_LBA))
                .with_filesystem(Some(Filesystem::Fat32))
                .with_root(Some("boot".to_owned())),
            // `B` boot partition.
            ImagePartition::new()
                .with_size(Some(NumBytes::mebibytes(128)))
                .with_ty(Some(mbr_types::FAT32_LBA)),
            // MBR extended partition.
            ImagePartition::new().with_ty(Some(mbr_types::EXTENDED)),
            // `A` system partition.
            ImagePartition::new()
                .with_ty(Some(mbr_types::LINUX))
                .with_filesystem(Some(Filesystem::Ext4))
                .with_root(Some("system".to_owned())),
        ]))
}

fn default_gpt_layout() -> ImageLayout {
    ImageLayout::new()
        .with_ty(Some(PartitionTableType::Gpt))
        .with_partitions(Some(vec![
            // Config partition.
            ImagePartition::new()
                .with_size(Some(NumBytes::mebibytes(256)))
                .with_ty(Some(gpt_types::EFI))
                .with_filesystem(Some(Filesystem::Fat32))
                .with_root(Some("config".to_owned())),
            // `A` boot partition.
            ImagePartition::new()
                .with_size(Some(NumBytes::mebibytes(256)))
                .with_ty(Some(gpt_types::LINUX))
                .with_filesystem(Some(Filesystem::Ext4))
                .with_root(Some("boot".to_owned())),
            // `B` boot partition.
            ImagePartition::new()
                .with_size(Some(NumBytes::mebibytes(256)))
                .with_ty(Some(gpt_types::LINUX)),
            // `A` system partition.
            ImagePartition::new()
                .with_ty(Some(gpt_types::LINUX))
                .with_filesystem(Some(Filesystem::Ext4))
                .with_root(Some("system".to_owned())),
        ]))
}
