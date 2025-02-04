use std::collections::HashMap;

use reportify::{Report, ResultExt};
use rugix_common::boot::grub::{grub_envblk_encode, GrubEnvError, RUGIX_BOOT_SPARE};
use tracing::info;

use crate::system::System;

pub fn set_spare_flag(system: &System) -> Result<(), Report<GrubEnvError>> {
    info!("setting spare flag for Grub boot flow");
    let mut envblk = HashMap::new();
    envblk.insert(RUGIX_BOOT_SPARE.to_owned(), "true".to_owned());
    let envblk = grub_envblk_encode(&envblk).whatever("unable to encode Grub environment")?;
    let config_partition = system
        .require_config_partition()
        .whatever("unable to get config partition")?;
    config_partition
        .ensure_writable(|| -> Result<(), Report<GrubEnvError>> {
            // It is safe to directly write to the file here. If the file is corrupt,
            // the system will simply boot from the default partition set. We still
            // need to use `rugpi` here to not break existing systems.
            std::fs::write(
                config_partition.path().join("rugpi/boot_spare.grubenv"),
                envblk,
            )
            .whatever("unable to write Grub environment")?;
            Ok(())
        })
        .whatever("unable to make config partition writable")??;
    Ok(())
}

pub fn clear_spare_flag(system: &System) -> Result<(), Report<GrubEnvError>> {
    info!("clearing spare flag for Grub boot flow");
    let mut envblk = HashMap::new();
    envblk.insert(RUGIX_BOOT_SPARE.to_owned(), "false".to_owned());
    let envblk = grub_envblk_encode(&envblk).whatever("unable to encode Grub environment")?;
    let config_partition = system
        .require_config_partition()
        .whatever("unable to get config partition")?;
    config_partition
        .ensure_writable(|| -> Result<(), Report<GrubEnvError>> {
            // It is safe to directly write to the file here. If the file is corrupt,
            // the system will simply boot from the default partition set.
            std::fs::write(
                config_partition.path().join("rugpi/boot_spare.grubenv"),
                envblk,
            )
            .whatever("unable to write Grub environment")?;
            Ok(())
        })
        .whatever("unable to make config partition writable")??;
    Ok(())
}
