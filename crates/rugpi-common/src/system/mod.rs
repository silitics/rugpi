use anyhow::{anyhow, bail};
use boot_entries::{BootEntries, BootEntry, BootEntryIdx};
use boot_flows::BootFlow;
use config::{load_system_config, SystemConfig};
use partitions::ConfigPartition;
use root::{find_system_device, SystemRoot};
use slots::{SlotKind, SystemSlots};
use tracing::warn;

use crate::{disk::blkdev::BlockDevice, Anyhow};

pub mod boot_entries;
pub mod boot_flows;
pub mod config;
pub mod partitions;
pub mod paths;
pub mod root;
pub mod slots;

pub struct System {
    pub config: SystemConfig,
    pub device: Option<BlockDevice>,
    pub root: Option<SystemRoot>,

    slots: SystemSlots,
    boot_entries: BootEntries,
    active_boot_entry: Option<BootEntryIdx>,
    boot_flow: Box<dyn BootFlow>,
    config_partition: Option<ConfigPartition>,
}

impl System {
    pub fn initialize() -> Anyhow<Self> {
        let system_config = load_system_config()?;
        let system_device = find_system_device();
        let system_root = system_device
            .as_ref()
            .and_then(SystemRoot::from_system_device);

        let config_partition = ConfigPartition::from_config(&system_config.config_partition);
        let Some(config_partition) = config_partition else {
            bail!("config partition cannot currently be disabled");
        };
        let slots = SystemSlots::from_config(system_root.as_ref(), system_config.slots.as_ref())?;
        let boot_entries = BootEntries::from_config(&slots, system_config.boot_groups.as_ref())?;
        // Mark boot entries and slots active.
        let mut active_boot_entry = None;
        for (idx, entry) in boot_entries.iter() {
            for (_, slot) in entry.slots() {
                let SlotKind::Block(raw) = &slots[slot].kind();
                if Some(raw.device()) == system_device.as_ref() {
                    entry.mark_active();
                    break;
                }
                /* TODO: Also look at `/proc/cmdline` to allow setting the active boot
                entry explicitly via a flag `rugpi.boot-entry=...`. For compatibility
                with RAUC, it makes sense to also look at `rauc.slot=...`. This holds
                the name of a RAUC slot, which we could directly map to a Rugpi slot
                assuming that the configuration preserves these names, e.g.:

                [slots."rootfs.0"]
                partition = 2

                [slots."rootfs.1"]
                partition = 3

                [boot-entries.A]
                slots = { rootfs = "rootfs.0" }

                [boot-entries.B]
                slots = { rootfs = "rootfs.1" }
                */
            }
            if entry.active() {
                active_boot_entry = Some(idx);
                // If the entry is active, then so are all its slots.
                for (_, slot) in entry.slots() {
                    slots[slot].mark_active();
                }
                break;
            }
        }
        if active_boot_entry.is_none() {
            warn!("unable to determine active boot entry");
        }
        let boot_flow = boot_flows::from_config(
            system_config.boot_flow.as_ref(),
            &config_partition,
            &boot_entries,
        )?;
        Ok(Self {
            config: system_config,
            device: system_device,
            root: system_root,
            slots,
            boot_entries,
            active_boot_entry,
            boot_flow,
            config_partition: Some(config_partition),
        })
    }

    pub fn root(&self) -> &Option<SystemRoot> {
        &self.root
    }

    pub fn config(&self) -> &SystemConfig {
        &self.config
    }

    pub fn slots(&self) -> &SystemSlots {
        &self.slots
    }

    pub fn boot_entries(&self) -> &BootEntries {
        &self.boot_entries
    }

    pub fn active_boot_entry(&self) -> Option<BootEntryIdx> {
        self.active_boot_entry
    }

    /// First entry that is not the default.
    pub fn spare_entry(&self) -> Anyhow<Option<(BootEntryIdx, &BootEntry)>> {
        let default = self.boot_flow.get_default(self)?;
        Ok(self.boot_entries().iter().find(|(idx, _)| *idx != default))
    }

    pub fn needs_commit(&self) -> Anyhow<bool> {
        Ok(self.active_boot_entry != Some(self.boot_flow.get_default(self)?))
    }

    pub fn boot_flow(&self) -> &dyn BootFlow {
        &*self.boot_flow
    }

    pub fn config_partition(&self) -> Option<&ConfigPartition> {
        self.config_partition.as_ref()
    }

    pub fn require_config_partition(&self) -> Anyhow<&ConfigPartition> {
        self.config_partition()
            .ok_or_else(|| anyhow!("config partition is required"))
    }

    pub fn commit(&self) -> Anyhow<()> {
        self.boot_flow.commit(self)
    }
}
