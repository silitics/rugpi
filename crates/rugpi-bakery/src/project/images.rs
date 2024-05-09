use rugpi_common::boot::BootFlow;
use serde::{Deserialize, Serialize};

use super::config::{Architecture, IncludeFirmware};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageConfig {
    /// The layer to use for the image.
    pub layer: String,
    /// Indicates whether to include firmware files in the image.
    #[serde(default)]
    pub include_firmware: IncludeFirmware,
    /// The target architecture to build an image for.
    #[serde(default)]
    pub architecture: Architecture,
    /// Indicates which boot flow to use for the image.
    #[serde(default)]
    pub boot_flow: BootFlow,
    pub layout: Option<ImageLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageLayout {
    pub partitions: Vec<ImagePartition>,
}

impl ImageLayout {
    pub fn sfdisk_render(&self) -> String {
        let mut partitions = Vec::new();

        for partition in &self.partitions {
            let mut fields = String::new();
            let mut is_first = true;

            if let Some(kind) = partition.kind.as_deref() {
                fields.push_str("type=");
                fields.push_str(kind);
                is_first = false;
            }
            if let Some(size) = &partition.size {
                if !is_first {
                    fields.push_str(", ");
                }
                fields.push_str("size=");
                fields.push_str(size);
            }

            partitions.push(fields);
        }

        let partitions = partitions.join("\n");

        indoc::formatdoc! { r#"
            label: dos
            unit: sectors
            grain: 4M
            
            {partitions}
        "# }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePartition {
    pub size: Option<String>,
    pub filesystem: Option<Filesystem>,
    pub path: Option<String>,
    pub label: Option<String>,
    pub kind: Option<String>,
}

impl ImagePartition {
    pub fn new() -> Self {
        Self {
            size: None,
            filesystem: None,
            path: None,
            label: None,
            kind: None,
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

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filesystem {
    Ext4,
    Fat32,
}

pub fn pi_image_layout() -> ImageLayout {
    ImageLayout {
        partitions: vec![
            ImagePartition::new()
                .with_size("256M")
                .with_kind("0c")
                .with_filesystem(Filesystem::Fat32)
                .with_label("config"),
            ImagePartition::new()
                .with_size("128M")
                .with_kind("0c")
                .with_filesystem(Filesystem::Fat32)
                .with_label("boot-a")
                .with_path("boot"),
            ImagePartition::new()
                .with_size("128M")
                .with_kind("0c")
                .with_filesystem(Filesystem::Fat32)
                .with_label("boot-b"),
            ImagePartition::new().with_kind("05").with_label("extended"),
            ImagePartition::new()
                .with_kind("83")
                .with_filesystem(Filesystem::Ext4)
                .with_label("system-a")
                .with_path(""),
        ],
    }
}
