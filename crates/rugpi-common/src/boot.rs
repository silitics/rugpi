use serde::{Deserialize, Serialize};

pub mod grub;
pub mod tryboot;
pub mod uboot;

/// Rugpi boot flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootFlow {
    /// Use the `tryboot` feature for booting and partition switching.
    #[default]
    Tryboot,
    /// Use U-Boot for booting and partition switching.
    #[serde(rename = "u-boot")]
    UBoot,
    #[serde(rename = "none")]
    None,
}

impl BootFlow {
    /// The string representation of the boot flow.
    pub fn as_str(self) -> &'static str {
        match self {
            BootFlow::Tryboot => "tryboot",
            BootFlow::UBoot => "u-boot",
            BootFlow::None => "none",
        }
    }
}
