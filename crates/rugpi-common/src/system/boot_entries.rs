use std::sync::atomic::{self, AtomicBool};

use anyhow::bail;
use indexmap::IndexMap;

use super::{
    config::BootEntriesConfig,
    slots::{SlotIdx, SystemSlots},
};
use crate::Anyhow;

/// Unique index of a boot entry of a system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BootEntryIdx {
    /// Index into the boot entry vector.
    idx: usize,
}

#[derive(Debug)]
pub struct BootEntries {
    entries: Vec<BootEntry>,
}

impl BootEntries {
    pub fn from_config(slots: &SystemSlots, config: Option<&BootEntriesConfig>) -> Anyhow<Self> {
        let mut entries = Vec::new();
        match config {
            Some(config) => {
                for (entry_name, entry_config) in config {
                    let mut map = IndexMap::new();
                    for (alias, name) in &entry_config.slots {
                        let Some((idx, _)) = slots.find_by_name(name) else {
                            bail!("slot {name} does not exist");
                        };
                        map.insert(alias.to_owned(), idx);
                    }
                    entries.push(BootEntry {
                        name: entry_name.to_owned(),
                        slots: map,
                        active: AtomicBool::new(false),
                    })
                }
            }
            None => {
                // Create Rugpi default boot entries.
                for (entry_name, entry_slots) in [
                    ("a", [("boot", "boot-a"), ("system", "system-a")]),
                    ("b", [("boot", "boot-b"), ("system", "system-b")]),
                ] {
                    let mut map = IndexMap::new();
                    for (alias, name) in entry_slots {
                        let Some((idx, _)) = slots.find_by_name(name) else {
                            bail!("slot {name} does not exist");
                        };
                        map.insert(alias.to_owned(), idx);
                    }
                    entries.push(BootEntry {
                        name: entry_name.to_owned(),
                        slots: map,
                        active: AtomicBool::new(false),
                    })
                }
            }
        }
        Ok(Self { entries })
    }

    pub fn iter(&self) -> impl Iterator<Item = (BootEntryIdx, &BootEntry)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| (BootEntryIdx { idx }, entry))
    }
}

#[derive(Debug)]
pub struct BootEntry {
    name: String,
    slots: IndexMap<String, SlotIdx>,
    active: AtomicBool,
}

impl BootEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn slots(&self) -> impl Iterator<Item = (&str, SlotIdx)> {
        self.slots.iter().map(|(name, idx)| (name.as_str(), *idx))
    }

    pub fn active(&self) -> bool {
        self.active.load(atomic::Ordering::Acquire)
    }

    pub fn mark_active(&self) {
        self.active.store(true, atomic::Ordering::Release);
    }
}
