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
}
