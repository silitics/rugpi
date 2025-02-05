use std::path::PathBuf;

use reportify::{bail, ResultExt};
use serde::{Deserialize, Serialize};
use xscript::{read_str, Run};

use super::BootFlow;

/// Custom boot flow implementation.
#[derive(Debug)]
pub struct CustomBootFlow {
    /// Path to the boot flow executable.
    pub(super) path: PathBuf,
}

impl BootFlow for CustomBootFlow {
    fn name(&self) -> &str {
        "custom"
    }

    fn set_try_next(
        &self,
        system: &crate::system::System,
        group: crate::system::boot_groups::BootGroupIdx,
    ) -> super::BootFlowResult<()> {
        let name = system.boot_entries()[group].name();
        let output = read_str!([&self.path, "set_try_next", name])
            .whatever("error running `set_try_next` on custom boot flow")?;
        serde_json::from_str::<OutputNone>(&output)
            .whatever("invalid output produced by custom boot flow")?;
        Ok(())
    }

    fn get_default(
        &self,
        system: &crate::system::System,
    ) -> super::BootFlowResult<crate::system::boot_groups::BootGroupIdx> {
        let output = read_str!([&self.path, "get_default"])
            .whatever("error running `get_default` on custom boot flow")?;
        let output = serde_json::from_str::<OutputGroup>(&output)
            .whatever("invalid output produced by custom boot flow")?;
        if let Some((idx, _)) = system.boot_entries().find_by_name(&output.group) {
            Ok(idx)
        } else {
            bail!(
                "custom boot flow returned unknown boot group {:?}",
                &output.group
            );
        }
    }

    fn commit(&self, system: &crate::system::System) -> super::BootFlowResult<()> {
        let name = system.boot_entries()[system.active_boot_entry().unwrap()].name();
        let output = read_str!([&self.path, "commit", name])
            .whatever("error running `commit` on custom boot flow")?;
        serde_json::from_str::<OutputNone>(&output)
            .whatever("invalid output produced by custom boot flow")?;
        Ok(())
    }

    fn pre_install(
        &self,
        system: &crate::system::System,
        group: crate::system::boot_groups::BootGroupIdx,
    ) -> super::BootFlowResult<()> {
        let name = system.boot_entries()[group].name();
        let output = read_str!([&self.path, "pre_install", name])
            .whatever("error running `pre_install` on custom boot flow")?;
        serde_json::from_str::<OutputNone>(&output)
            .whatever("invalid output produced by custom boot flow")?;
        Ok(())
    }

    fn post_install(
        &self,
        system: &crate::system::System,
        group: crate::system::boot_groups::BootGroupIdx,
    ) -> super::BootFlowResult<()> {
        let name = system.boot_entries()[group].name();
        let output = read_str!([&self.path, "post_install", name])
            .whatever("error running `post_install` on custom boot flow")?;
        serde_json::from_str::<OutputNone>(&output)
            .whatever("invalid output produced by custom boot flow")?;
        Ok(())
    }
}

/// Output type for operations that output a boot group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputGroup {
    group: String,
}

/// Output type for operations that do not provide any output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputNone {}
