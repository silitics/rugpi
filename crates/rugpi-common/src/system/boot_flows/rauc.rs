//! RAUC-compatible boot flows.

use std::path::{Path, PathBuf};

use reportify::ResultExt;
use serde::{Deserialize, Serialize};

use super::BootFlowResult;
use crate::boot::grub::{load_grub_env, save_grub_env, GrubEnv};
use crate::system::boot_flows::{BootFlow, BootGroupStatus};
use crate::system::boot_groups::BootGroupIdx;
use crate::system::System;

/// RAUC-compatible Grub boot flow configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub struct RaucGrubBootFlowConfig {
    /// Path to the Grub environment file.
    grub_env: Option<PathBuf>,
}

/// RAUC-compatible Grub boot flow.
#[derive(Debug)]
pub struct RaucGrubBootFlow {
    /// Path to the Grub environment file.
    grub_env_path: PathBuf,
}

impl RaucGrubBootFlow {
    /// Create boot flow from the given configuration.
    #[allow(dead_code)]
    pub fn from_config(config: &RaucGrubBootFlowConfig) -> BootFlowResult<Self> {
        let grub_env_path = config
            .grub_env
            .as_deref()
            .unwrap_or(Path::new("/boot/grub/grubenv"))
            .to_path_buf();
        let this = Self { grub_env_path };
        // Make sure that we can load the Grub environment from the provided path.
        this.load_grub_env()?;
        Ok(this)
    }

    fn load_grub_env(&self) -> BootFlowResult<GrubEnv> {
        load_grub_env(&self.grub_env_path).whatever("unable to load Grub environment")
    }

    #[allow(dead_code)]
    fn save_grub_env(&self, env: &GrubEnv) -> BootFlowResult<()> {
        save_grub_env(&self.grub_env_path, env).whatever("unable to save Grub environment")
    }
}

#[allow(unused_variables)]
impl BootFlow for RaucGrubBootFlow {
    fn set_try_next(&self, system: &System, entry: BootGroupIdx) -> BootFlowResult<()> {
        // RAUC's Grub integration does not allow setting oneshot entries. We make the
        // the requested entry the primary. If anything goes wrong, the system will revert
        // to the current system anyway as it is still in the boot order.
        todo!()
    }

    fn commit(&self, system: &System) -> BootFlowResult<()> {
        todo!()
    }

    fn get_default(&self, system: &System) -> BootFlowResult<BootGroupIdx> {
        todo!()
    }

    fn remaining_attempts(
        &self,
        system: &System,
        entry: BootGroupIdx,
    ) -> BootFlowResult<Option<u64>> {
        todo!()
    }

    fn get_status(&self, system: &System, entry: BootGroupIdx) -> BootFlowResult<BootGroupStatus> {
        todo!()
    }

    fn mark_good(&self, system: &System, entry: BootGroupIdx) -> BootFlowResult<()> {
        todo!()
    }

    fn mark_bad(&self, system: &System, entry: BootGroupIdx) -> BootFlowResult<()> {
        todo!()
    }

    fn name(&self) -> &str {
        "rauc-grub"
    }
}
