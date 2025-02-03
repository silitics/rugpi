use std::ops::Index;
use std::sync::Mutex;

use indexmap::IndexMap;
use reportify::{bail, ResultExt};

use crate::config::system::{BlockSlotConfig, SlotConfig};

use super::root::SystemRoot;
use super::SystemResult;
use rugix_common::disk::blkdev::BlockDevice;

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
    fn from_iter<'i, I>(root: Option<&SystemRoot>, iter: I) -> SystemResult<Self>
    where
        I: Iterator<Item = (&'i str, &'i SlotConfig)>,
    {
        let mut slots = Vec::new();
        for (name, config) in iter {
            let SlotConfig::Block(raw) = &config;
            let device = if let Some(device) = &raw.device {
                BlockDevice::new(device)
                    .whatever("slot device is not a block device")
                    .with_info(|_| format!("device: {device:?}"))?
            } else if let Some(partition) = &raw.partition {
                let Some(root) = root else {
                    bail!("no system root")
                };
                let Some(device) = root.resolve_partition(*partition) else {
                    bail!("partition {partition} for slot {name:?} not found");
                };
                device
            } else {
                bail!("invalid configuration: no device and partition for {name}");
            };
            slots.push(Slot::new(
                name.to_owned(),
                SlotKind::Block(BlockSlot { device }),
                config.clone(),
            ))
        }
        Ok(Self { slots })
    }

    pub fn from_config(
        root: Option<&SystemRoot>,
        config: Option<&IndexMap<String, SlotConfig>>,
    ) -> SystemResult<Self> {
        match config {
            Some(config) => Self::from_iter(
                root,
                config.iter().map(|(name, config)| (name.as_str(), config)),
            ),
            None => {
                let Some(root) = root else {
                    bail!("no system root")
                };
                let Some(table) = &root.table else {
                    bail!("unable to determine slots: no table");
                };
                let default_slots = if table.is_mbr() {
                    DEFAULT_MBR_SLOTS
                } else {
                    DEFAULT_GPT_SLOTS
                };
                Self::from_iter(
                    Some(root),
                    default_slots.iter().map(|(name, config)| (*name, config)),
                )
            }
        }
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
    _config: SlotConfig,
    active: Mutex<bool>,
}

impl Slot {
    /// Create a new slot.
    fn new(name: String, kind: SlotKind, config: SlotConfig) -> Self {
        Self {
            name,
            kind,
            _config: config,
            active: Mutex::new(false),
        }
    }

    /// Name of the slot.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Kind of the slot.
    pub fn kind(&self) -> &SlotKind {
        &self.kind
    }

    /// Indicates whether the slot is active.
    pub fn active(&self) -> bool {
        *self.active.lock().unwrap()
    }

    /// Indicates whether the slot is of type `block`.
    pub fn is_block(&self) -> bool {
        matches!(self.kind, SlotKind::Block(_))
    }

    /// Mark the slot as active.
    pub fn mark_active(&self) {
        *self.active.lock().unwrap() = true;
    }
}

#[derive(Debug)]
pub enum SlotKind {
    Block(BlockSlot),
}

#[derive(Debug)]
pub struct BlockSlot {
    device: BlockDevice,
}

impl BlockSlot {
    pub fn device(&self) -> &BlockDevice {
        &self.device
    }
}

/// Default slots of an MBR-partitioned root device.
const DEFAULT_MBR_SLOTS: &[(&str, SlotConfig)] = &[
    ("boot-a", default_slot_config(2)),
    ("boot-b", default_slot_config(3)),
    ("system-a", default_slot_config(5)),
    ("system-b", default_slot_config(6)),
];

/// Default slots of a GPT-partitioned root device.
const DEFAULT_GPT_SLOTS: &[(&str, SlotConfig)] = &[
    ("boot-a", default_slot_config(2)),
    ("boot-b", default_slot_config(3)),
    ("system-a", default_slot_config(4)),
    ("system-b", default_slot_config(5)),
];

/// Configuration of default slots for the given partition.
const fn default_slot_config(partition: u32) -> SlotConfig {
    SlotConfig::Block(BlockSlotConfig {
        device: None,
        partition: Some(partition),
    })
}
