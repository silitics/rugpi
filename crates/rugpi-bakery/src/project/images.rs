use rugpi_common::{boot::BootFlow, disk::PartitionType};
use serde::{Deserialize, Serialize};

use super::config::{Architecture, IncludeFirmware};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageConfig {
    /// The layer to use for the image.
    pub layer: String,
    /// Indicates whether to include firmware files in the image.
    pub include_firmware: Option<IncludeFirmware>,
    /// The target architecture to build an image for.
    #[serde(default)]
    pub architecture: Architecture,
    /// Indicates which boot flow to use for the image.
    pub boot_flow: Option<BootFlow>,
    pub size: Option<String>,
    pub layout: Option<ImageLayout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageLayout {
    #[serde(default, rename = "type")]
    pub ty: ImageLayoutKind,
    pub partitions: Vec<ImagePartition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePartition {
    pub size: Option<String>,
    pub filesystem: Option<Filesystem>,
    pub root: Option<String>,
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub ty: Option<PartitionType>,
}

impl ImagePartition {
    pub fn new() -> Self {
        Self {
            size: None,
            filesystem: None,
            root: None,
            label: None,
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

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_ty(mut self, ty: PartitionType) -> Self {
        self.ty = Some(ty);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImageLayoutKind {
    #[default]
    Mbr,
    Gpt,
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
        ty: ImageLayoutKind::Mbr,
        partitions: vec![
            ImagePartition::new()
                .with_size("256M")
                .with_ty(PartitionType::Mbr(0x0c))
                .with_filesystem(Filesystem::Fat32)
                .with_label("config"),
            ImagePartition::new()
                .with_size("128M")
                .with_ty(PartitionType::Mbr(0x0c))
                .with_filesystem(Filesystem::Fat32)
                .with_label("boot-a")
                .with_root("boot"),
            ImagePartition::new()
                .with_size("128M")
                .with_ty(PartitionType::Mbr(0x0c))
                .with_filesystem(Filesystem::Fat32)
                .with_label("boot-b"),
            ImagePartition::new()
                .with_ty(PartitionType::Mbr(0x05))
                .with_label("extended"),
            ImagePartition::new()
                .with_ty(PartitionType::Mbr(0x83))
                .with_filesystem(Filesystem::Ext4)
                .with_label("system-a")
                .with_root("system"),
        ],
    }
}
