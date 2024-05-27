use serde::Deserialize;

pub mod generic_grub_efi;
pub mod rpi_tryboot;
pub mod rpi_uboot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    GenericGrubEfi,
    RpiTryboot,
    RpiUboot,
    Unknown,
}
