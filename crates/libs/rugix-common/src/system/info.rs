use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    boot_flow: String,
    slots: IndexMap<String, SlotInfo>,
    boot_groups: IndexMap<String, BootGroupInfo>,
    default_boot_group: Option<String>,
    active_boot_group: Option<String>,
}

impl SystemInfo {
    pub fn from(system: &System) -> Self {
        let boot_flow = system.boot_flow().name().to_owned();
        let slots = system
            .slots()
            .iter()
            .map(|(_, slot)| {
                (
                    slot.name().to_owned(),
                    SlotInfo {
                        active: slot.active(),
                    },
                )
            })
            .collect();
        let active_boot_group = system
            .active_boot_entry()
            .map(|idx| system.boot_entries()[idx].name().to_owned());
        let default_boot_group = Some(
            system.boot_entries()[system.boot_flow().get_default(system).unwrap()]
                .name()
                .to_owned(),
        );
        let boot_groups = system
            .boot_entries()
            .iter()
            .map(|(_, group)| (group.name().to_owned(), BootGroupInfo {}))
            .collect();
        Self {
            boot_flow,
            slots,
            boot_groups,
            active_boot_group,
            default_boot_group,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootGroupInfo {}
