use std::{os::unix::prelude::FileTypeExt, path::Path};

macro_rules! sd_card_dev_const {
    ($name:ident, $part:literal) => {
        pub const $name: &str = concat!("/dev/mmcblk0", $part);
    };
}

sd_card_dev_const!(SD_CARD, "");
sd_card_dev_const!(SD_PART_CONFIG, "p1");
sd_card_dev_const!(SD_PART_BOOT_A, "p2");
sd_card_dev_const!(SD_PART_BOOT_B, "p3");
sd_card_dev_const!(SD_PART_SYSTEM_A, "p5");
sd_card_dev_const!(SD_PART_SYSTEM_B, "p6");
sd_card_dev_const!(SD_PART_DATA, "p7");

pub fn is_block_dev(dev: impl AsRef<Path>) -> bool {
    let dev = dev.as_ref();
    dev.metadata()
        .map(|metadata| metadata.file_type().is_block_device())
        .unwrap_or(false)
}

pub fn is_dir(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}
