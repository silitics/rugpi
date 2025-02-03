use reportify::{Report, ResultExt};
use rugix_common::boot::uboot::UBootEnv;

use crate::system::boot_flows::BootFlowError;
use crate::system::System;

pub fn set_spare_flag(system: &System) -> Result<(), Report<BootFlowError>> {
    let mut boot_spare_env = UBootEnv::new();
    boot_spare_env.set("boot_spare", "1");
    let config_partition = system
        .require_config_partition()
        .whatever("unable to get config partition")?;
    config_partition
        .ensure_writable(|| -> Result<(), Report<BootFlowError>> {
            // It is safe to directly write to the file here. If the file is corrupt,
            // the system will simply boot from the default partition set.
            boot_spare_env
                .save(config_partition.path().join("boot_spare.env"))
                .whatever("unable to save uboot environment")?;
            Ok(())
        })
        .whatever("unable to make config partition writable")??;
    Ok(())
}

pub fn clear_spare_flag(system: &System) -> Result<(), Report<BootFlowError>> {
    let mut boot_spare_env = UBootEnv::new();
    boot_spare_env.set("boot_spare", "0");
    let config_partition = system
        .require_config_partition()
        .whatever("unable to get config partition")?;
    config_partition
        .ensure_writable(|| -> Result<(), Report<BootFlowError>> {
            // It is safe to directly write to the file here. If the file is corrupt,
            // the system will simply boot from the default partition set.
            boot_spare_env
                .save(config_partition.path().join("boot_spare.env"))
                .whatever("unable to save uboot environment")?;
            Ok(())
        })
        .whatever("unable to make config partition writable")??;
    Ok(())
}
