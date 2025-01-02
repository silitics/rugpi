use rugpi_common::disk::gpt::gpt_types;
use rugpi_common::disk::{PartitionTableType, PartitionType};
use serde::{Deserialize, Serialize};

use super::config::Architecture;
use crate::bake::targets::Target;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageConfig {
    /// The layer to use for the image.
    pub layer: String,
    /// The target architecture to build an image for.
    pub architecture: Architecture,
    /// Indicates which boot flow to use for the image.
    pub target: Option<Target>,
    pub size: Option<String>,
    pub layout: Option<ImageLayout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageLayout {
    #[serde(default, rename = "type")]
    pub ty: PartitionTableType,
    pub partitions: Vec<ImagePartition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePartition {
    pub size: Option<String>,
    pub filesystem: Option<Filesystem>,
    pub root: Option<String>,
    #[serde(rename = "type")]
    pub ty: Option<PartitionType>,
}

impl ImagePartition {
    pub fn new() -> Self {
        Self {
            size: None,
            filesystem: None,
            root: None,
            ty: None,
        }
    }

    pub fn with_size(mut self, size: impl Into<String>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn with_filesystem(mut self, filesystem: Filesystem) -> Self {
        self.filesystem = Some(filesystem);
        self
    }

    pub fn with_root(mut self, path: impl Into<String>) -> Self {
        self.root = Some(path.into());
        self
    }

    pub fn with_ty(mut self, ty: PartitionType) -> Self {
        self.ty = Some(ty);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filesystem {
    Ext4,
    Fat32,
}

impl Filesystem {
    pub fn as_str(self) -> &'static str {
        match self {
            Filesystem::Ext4 => "Ext4",
            Filesystem::Fat32 => "FAT32",
        }
    }
}

pub fn pi_image_layout() -> ImageLayout {
    ImageLayout {
        ty: PartitionTableType::Mbr,
        partitions: vec![
            ImagePartition::new()
                .with_size("256M")
                .with_ty(PartitionType::Mbr(0x0c))
                .with_filesystem(Filesystem::Fat32)
                .with_root("config"),
            ImagePartition::new()
                .with_size("128M")
                .with_ty(PartitionType::Mbr(0x0c))
                .with_filesystem(Filesystem::Fat32)
                .with_root("boot"),
            ImagePartition::new()
                .with_size("128M")
                .with_ty(PartitionType::Mbr(0x0c))
                .with_filesystem(Filesystem::Fat32),
            ImagePartition::new().with_ty(PartitionType::Mbr(0x05)),
            ImagePartition::new()
                .with_ty(PartitionType::Mbr(0x83))
                .with_filesystem(Filesystem::Ext4)
                .with_root("system"),
        ],
    }
}

pub fn grub_efi_image_layout() -> ImageLayout {
    ImageLayout {
        ty: PartitionTableType::Gpt,
        partitions: vec![
            ImagePartition::new()
                .with_size("256M")
                .with_ty(gpt_types::EFI)
                .with_filesystem(Filesystem::Fat32)
                .with_root("config"),
            ImagePartition::new()
                .with_size("256M")
                .with_ty(gpt_types::LINUX)
                .with_filesystem(Filesystem::Ext4)
                .with_root("boot"),
            ImagePartition::new()
                .with_size("256M")
                .with_ty(gpt_types::LINUX),
            ImagePartition::new()
                .with_ty(gpt_types::LINUX)
                .with_filesystem(Filesystem::Ext4)
                .with_root("system"),
        ],
    }
}
