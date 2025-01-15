use std::path::Path;

use reportify::{whatever, ResultExt};

use rugix_blocking::{block, BlockingCtx, MaybeAborted};
use rugpi_common::boot::grub::grub_write_defaults;

use crate::config::images::ImageConfig;
use crate::config::projects::Architecture;
use crate::BakeryResult;

pub fn initialize_grub<'cx>(
    cx: BlockingCtx<'cx>,
    config: &ImageConfig,
    config_dir: &Path,
) -> MaybeAborted<'cx, BakeryResult<()>> {
    block!(rugix_fs::create_dir_recursive(
        cx,
        &config_dir.join("EFI/BOOT")
    ))
    .ok();
    block!(rugix_fs::create_dir_recursive(
        cx,
        &config_dir.join("rugpi")
    ))
    .ok();
    let mut copier = rugix_fs::Copier::new();
    block!(
        try {
            copier
                .copy_file(
                    cx,
                    "/usr/share/rugpi/boot/grub/cfg/first.grub.cfg".as_ref(),
                    &config_dir.join("rugpi/grub.cfg"),
                )
                .map(|result| result.whatever("unable to copy first stage boot script"))
        }
    );
    block!(
        try {
            grub_write_defaults(config_dir).whatever("unable to write Grub default environment")
        }
    );
    match config.architecture {
        Architecture::Arm64 => {
            block!(
                try {
                    copier
                        .copy_file(
                            cx,
                            "/usr/share/rugpi/boot/grub/bin/BOOTAA64.efi".as_ref(),
                            &config_dir.join("EFI/BOOT/BOOTAA64.efi"),
                        )
                        .map(|result| result.whatever("unable to copy Grub binary"))
                }
            );
        }
        Architecture::Amd64 => {
            block!(
                try {
                    copier
                        .copy_file(
                            cx,
                            "/usr/share/rugpi/boot/grub/bin/BOOTX64.efi".as_ref(),
                            &config_dir.join("EFI/BOOT/BOOTX64.efi"),
                        )
                        .map(|result| result.whatever("unable to copy Grub binary"))
                }
            );
        }
        _ => {
            block!(try Err(whatever!(
                "no Grub support for architecture `{}`",
                config.architecture.as_str()
            )));
        }
    }
    MaybeAborted::Done(Ok(()))
}
