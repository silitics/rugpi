use std::path::Path;

use reportify::{whatever, Report, ResultExt};

use rugix_tasks::{BlockingCtx, MaybeAborted};
use rugix_try::xtry;
use rugpi_common::boot::grub::grub_write_defaults;

use crate::config::images::ImageConfig;
use crate::config::projects::Architecture;
use crate::BakeryError;

type AbortedBakeryResult<T> = MaybeAborted<Result<T, Report<BakeryError>>>;

pub fn initialize_grub<'cx>(
    cx: BlockingCtx<'cx>,
    config: &ImageConfig,
    config_dir: &Path,
) -> AbortedBakeryResult<()> {
    xtry!(rugix_fs::create_dir_recursive(
        cx,
        &config_dir.join("EFI/BOOT")
    ))
    .ok();
    xtry!(rugix_fs::create_dir_recursive(
        cx,
        &config_dir.join("rugpi")
    ))
    .ok();
    let mut copier = rugix_fs::Copier::new();
    xtry!(xtry!(copier
        .copy_file(
            cx,
            "/usr/share/rugpi/boot/grub/cfg/first.grub.cfg".as_ref(),
            &config_dir.join("rugpi/grub.cfg"),
        )
        .map(
            |result| result.whatever("unable to copy first stage boot script")
        )));
    xtry!(grub_write_defaults(config_dir).whatever("unable to write Grub default environment"));
    match config.architecture {
        Architecture::Arm64 => {
            xtry!(xtry!(copier
                .copy_file(
                    cx,
                    "/usr/share/rugpi/boot/grub/bin/BOOTAA64.efi".as_ref(),
                    &config_dir.join("EFI/BOOT/BOOTAA64.efi"),
                )
                .map(|result| result.whatever("unable to copy Grub binary"))));
        }
        Architecture::Amd64 => {
            xtry!(xtry!(copier
                .copy_file(
                    cx,
                    "/usr/share/rugpi/boot/grub/bin/BOOTX64.efi".as_ref(),
                    &config_dir.join("EFI/BOOT/BOOTX64.efi"),
                )
                .map(|result| result.whatever("unable to copy Grub binary"))));
        }
        _ => {
            return MaybeAborted::Done(Err(whatever!(
                "no Grub support for architecture `{}`",
                config.architecture.as_str()
            )));
        }
    }
    MaybeAborted::Done(Ok(()))
}
