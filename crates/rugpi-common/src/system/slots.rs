use std::{
    ops::Index,
    sync::atomic::{self, AtomicBool},
};

use anyhow::bail;

use super::{
    config::{SlotConfigKind, SlotsConfig},
    SystemRoot,
};
use crate::{disk::blkdev::BlockDevice, Anyhow};

/// Unique index of a slot of a system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotIdx {
    /// Index into the slot vector.
    idx: usize,
}

/// Slots of a system.
pub struct SystemSlots {
    /// Slots of the system.
    slots: Vec<Slot>,
}

impl SystemSlots {
    pub fn from_config(root: &SystemRoot, config: Option<&SlotsConfig>) -> Anyhow<Self> {
        let mut slots = Vec::new();
        match config {
            Some(config) => {
                for (name, config) in config {
                    let SlotConfigKind::Raw(raw) = &config.kind;
                    let device = if let Some(device) = &raw.device {
                        BlockDevice::new(device)?
                    } else if let Some(partition) = &raw.partition {
                        let Some(device) = root.resolve_partition(*partition)? else {
                            bail!("partition {partition} for slot {name:?} not found");
                        };
                        device
                    } else {
                        bail!("invalid configuration: no device and partition for {name}");
                    };
                    slots.push(
                        Slot::new(name.clone(), SlotKind::Raw(RawSlot { device }))
                            .with_protected(config.protected),
                    )
                }
            }
            None => {
                let Some(table) = &root.table else {
                    bail!("unable to determine slots: no table");
                };
                let default_slots = if table.is_mbr() {
                    DEFAULT_MBR_SLOTS
                } else {
                    DEFAULT_GPT_SLOTS
                };
                for (name, partition) in default_slots {
                    let Some(device) = root.resolve_partition(*partition)? else {
                        bail!("partition {partition} for slot {name:?} not found");
                    };
                    slots.push(Slot::new(
                        (*name).to_owned(),
                        SlotKind::Raw(RawSlot { device }),
                    ));
                }
            }
        }
        Ok(Self { slots })
    }

    /// Find a slot by its name.
    pub fn find_by_name(&self, name: &str) -> Option<(SlotIdx, &Slot)> {
        // There are only a few slots, so we can get away with linear search.
        self.iter().find(|(_, slot)| slot.name == name)
    }

    /// Iterator of the slots.
    pub fn iter(&self) -> impl Iterator<Item = (SlotIdx, &Slot)> {
        self.slots
            .iter()
            .enumerate()
            .map(|(idx, slot)| (SlotIdx { idx }, slot))
    }
}

impl Index<SlotIdx> for SystemSlots {
    type Output = Slot;

    fn index(&self, index: SlotIdx) -> &Self::Output {
        &self.slots[index.idx]
    }
}

#[derive(Debug)]
pub struct Slot {
    name: String,
    kind: SlotKind,
    protected: bool,
    active: AtomicBool,
}

impl Slot {
    pub fn new(name: String, kind: SlotKind) -> Self {
        Self {
            name,
            kind,
            protected: false,
            active: AtomicBool::new(false),
        }
    }

    pub fn with_protected(mut self, protected: bool) -> Self {
        self.protected = protected;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &SlotKind {
        &self.kind
    }

    pub fn protected(&self) -> bool {
        self.protected
    }

    pub fn active(&self) -> bool {
        self.active.load(atomic::Ordering::Acquire)
    }

    pub fn is_raw(&self) -> bool {
        matches!(self.kind, SlotKind::Raw(_))
    }

    pub fn mark_active(&self) {
        self.active.store(true, atomic::Ordering::Release);
    }
}

#[derive(Debug)]
pub enum SlotKind {
    Raw(RawSlot),
}

#[derive(Debug)]
pub struct RawSlot {
    device: BlockDevice,
}

impl RawSlot {
    pub fn device(&self) -> &BlockDevice {
        &self.device
    }
}

/// Default slots of an MBR-partitioned disk.
const DEFAULT_MBR_SLOTS: &[(&str, u32)] = &[
    ("boot-a", 2),
    ("boot-b", 3),
    ("system-a", 5),
    ("system-b", 6),
];

/// Default slots of a GPT-partitioned disk.
const DEFAULT_GPT_SLOTS: &[(&str, u32)] = &[
    ("boot-a", 2),
    ("boot-b", 3),
    ("system-a", 4),
    ("system-b", 5),
];
