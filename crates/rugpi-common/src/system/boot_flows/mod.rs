//! Boot flows for atomic system updates.

use std::{collections::HashMap, fmt::Debug, fs::File, io::Write};

use anyhow::bail;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use super::{
    boot_groups::{BootGroupIdx, BootGroups},
    config::BootFlowConfig,
    slots::SlotIdx,
    ConfigPartition, System,
};
use crate::{
    boot::{
        grub::{self, load_grub_env, write_with_hash, RUGPI_BOOTPART},
        tryboot::{self, AutobootSection, AUTOBOOT_A, AUTOBOOT_B},
        uboot::{self, UBootEnv},
    },
    grub_patch_env,
    mount::Mounted,
    partitions::get_disk_id,
    rpi_patch_boot,
    system::slots::SlotKind,
    utils::ascii_numbers,
    Anyhow,
};

#[cfg(feature = "compat-mender")]
pub(super) mod mender;
#[cfg(feature = "compat-rauc")]
pub(super) mod rauc;

/// Implementation of a boot flow.
pub trait BootFlow: Debug {
    /// Name of the boot flow.
    fn name(&self) -> &str;

    /// Set the boot group to try on the next boot.
    ///
    /// If booting fails, the bootloader should fallback to the previous default.
    ///
    /// Note that this function may change the default boot group.
    fn set_try_next(&self, system: &System, group: BootGroupIdx) -> Anyhow<()>;

    /// Get the default boot group.
    fn get_default(&self, system: &System) -> Anyhow<BootGroupIdx>;

    /// Make the active boot group the default.
    fn commit(&self, system: &System) -> Anyhow<()>;

    /// Called prior to installing an update to the given boot group.
    #[allow(unused_variables)]
    fn pre_install(&self, system: &System, group: BootGroupIdx) -> Anyhow<()> {
        Ok(())
    }

    /// Called after installing an update to the given boot group.
    #[allow(unused_variables)]
    fn post_install(&self, system: &System, group: BootGroupIdx) -> Anyhow<()> {
        Ok(())
    }

    /// Get the number of remaining attempts for the given boot group.
    ///
    /// Returns [`None`] in case there is an unlimited number of attempts.
    #[allow(unused_variables)]
    fn remaining_attempts(&self, system: &System, group: BootGroupIdx) -> Anyhow<Option<u64>> {
        Ok(None)
    }

    /// Get the status of the boot group.
    #[allow(unused_variables)]
    fn get_status(&self, system: &System, group: BootGroupIdx) -> Anyhow<BootGroupStatus> {
        Ok(BootGroupStatus::Unknown)
    }

    /// Mark a boot group as good.
    #[allow(unused_variables)]
    fn mark_good(&self, system: &System, group: BootGroupIdx) -> Anyhow<()> {
        Ok(())
    }

    /// Mark a boot group as bad.
    #[allow(unused_variables)]
    fn mark_bad(&self, system: &System, group: BootGroupIdx) -> Anyhow<()> {
        Ok(())
    }
}

/// Boot group status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BootGroupStatus {
    /// Status is unknown.
    #[default]
    Unknown,
    /// Boot group is known to be good (bootable and working).
    Good,
    /// Boot group is known to be bad (should not be booted).
    Bad,
}

pub fn from_config(
    config: Option<&BootFlowConfig>,
    config_partition: &ConfigPartition,
    boot_entries: &BootGroups,
) -> Anyhow<Box<dyn BootFlow>> {
    assert!(config.is_none(), "config not supported yet");
    let mut entries = boot_entries.iter();
    let Some((entry_a_idx, entry_a)) = entries.next() else {
        bail!("invalid number of entries");
    };
    let Some((entry_b_idx, entry_b)) = entries.next() else {
        bail!("invalid number of entries");
    };
    let Some(boot_a) = entry_a.get_slot("boot") else {
        bail!("unable to get A boot slot");
    };
    let Some(boot_b) = entry_b.get_slot("boot") else {
        bail!("unable to get B boot slot");
    };
    let Some(system_a) = entry_a.get_slot("system") else {
        bail!("unable to get A system slot");
    };
    let Some(system_b) = entry_b.get_slot("system") else {
        bail!("unable to get B system slot");
    };
    let inner = RugpiBootFlow {
        entry_a: entry_a_idx,
        entry_b: entry_b_idx,
        boot_a,
        boot_b,
        system_a,
        system_b,
    };
    if config_partition.path().join("autoboot.txt").exists() {
        Ok(Box::new(Tryboot { inner }))
    } else if config_partition
        .path()
        .join("bootpart.default.env")
        .exists()
    {
        Ok(Box::new(UBoot { inner }))
    } else if config_partition
        .path()
        .join("rugpi/primary.grubenv")
        .exists()
        && config_partition.path().join("EFI").is_dir()
    {
        Ok(Box::new(GrubEfi { inner }))
    } else {
        bail!("unable to detect boot flow");
    }
}

#[derive(Debug)]
struct RugpiBootFlow {
    entry_a: BootGroupIdx,
    entry_b: BootGroupIdx,
    boot_a: SlotIdx,
    boot_b: SlotIdx,
    system_a: SlotIdx,
    system_b: SlotIdx,
}

#[derive(Debug)]
struct Tryboot {
    inner: RugpiBootFlow,
}

impl BootFlow for Tryboot {
    fn set_try_next(&self, system: &System, entry: BootGroupIdx) -> Anyhow<()> {
        if entry != self.get_default(system)? {
            tryboot::set_spare_flag()?;
        } else {
            tryboot::clear_spare_flag()?;
        }
        Ok(())
    }

    fn commit(&self, system: &System) -> Anyhow<()> {
        let config_partition = system.require_config_partition()?;
        config_partition.ensure_writable(|| {
            let autoboot_new_path = config_partition.path().join("autoboot.txt.new");
            let mut autoboot_new = File::create(&autoboot_new_path)?;
            autoboot_new.write_all(
                if system.active_boot_entry() == Some(self.inner.entry_a) {
                    AUTOBOOT_A
                } else if system.active_boot_entry() == Some(self.inner.entry_b) {
                    AUTOBOOT_B
                } else {
                    panic!("should never happen");
                }
                .as_bytes(),
            )?;
            autoboot_new.flush()?;
            autoboot_new.sync_all()?;
            drop(autoboot_new);
            std::fs::rename(
                autoboot_new_path,
                config_partition.path().join("autoboot.txt"),
            )?;
            Ok(())
        })?
    }

    fn get_default(&self, system: &System) -> Anyhow<BootGroupIdx> {
        let autoboot_txt = std::fs::read_to_string(
            system
                .require_config_partition()?
                .path()
                .join("autoboot.txt"),
        )?;
        let mut section = AutobootSection::Unknown;
        for line in autoboot_txt.lines() {
            if line.starts_with("[all]") {
                section = AutobootSection::All;
            } else if line.starts_with("[tryboot]") {
                section = AutobootSection::Tryboot;
            } else if line.starts_with('[') {
                section = AutobootSection::Unknown;
            } else if line.starts_with("boot_partition=2") && section == AutobootSection::All {
                return Ok(self.inner.entry_a);
            } else if line.starts_with("boot_partition=3") && section == AutobootSection::All {
                return Ok(self.inner.entry_b);
            }
        }
        bail!("unable to determine partition set from `autoboot.txt`");
    }

    fn post_install(&self, system: &System, entry: BootGroupIdx) -> Anyhow<()> {
        tryboot_uboot_post_install(&self.inner, system, entry)
    }

    fn name(&self) -> &str {
        "tryboot"
    }
}

#[derive(Debug)]
struct UBoot {
    inner: RugpiBootFlow,
}

impl BootFlow for UBoot {
    fn set_try_next(&self, system: &System, entry: BootGroupIdx) -> Anyhow<()> {
        if entry != self.get_default(system)? {
            uboot::set_spare_flag(system)?;
        } else {
            uboot::clear_spare_flag(system)?;
        }
        Ok(())
    }

    fn commit(&self, system: &System) -> Anyhow<()> {
        let config_partition = system.require_config_partition()?;
        config_partition.ensure_writable(|| {
            let mut bootpart_env = UBootEnv::new();
            if system.active_boot_entry() == Some(self.inner.entry_a) {
                bootpart_env.set("bootpart", "2")
            } else if system.active_boot_entry() == Some(self.inner.entry_b) {
                bootpart_env.set("bootpart", "3");
            } else {
                panic!("should never happen");
            };
            let new_path = config_partition.path().join("bootpart.default.env.new");
            bootpart_env.save(&new_path)?;
            File::open(&new_path)?.sync_all()?;
            std::fs::rename(
                new_path,
                config_partition.path().join("bootpart.default.env"),
            )?;
            Ok(())
        })?
    }

    fn get_default(&self, system: &System) -> Anyhow<BootGroupIdx> {
        let config_partition = system.require_config_partition()?;
        let bootpart_env = UBootEnv::load(config_partition.path().join("bootpart.default.env"))?;
        let Some(bootpart) = bootpart_env.get("bootpart") else {
            bail!("Invalid bootpart environment.");
        };
        if bootpart == "2" {
            Ok(self.inner.entry_a)
        } else if bootpart == "3" {
            Ok(self.inner.entry_b)
        } else {
            bail!("Invalid default `bootpart`.");
        }
    }

    fn post_install(&self, system: &System, entry: BootGroupIdx) -> Anyhow<()> {
        tryboot_uboot_post_install(&self.inner, system, entry)
    }

    fn name(&self) -> &str {
        "u-boot"
    }
}

fn tryboot_uboot_post_install(
    inner: &RugpiBootFlow,
    system: &System,
    entry: BootGroupIdx,
) -> Anyhow<()> {
    let temp_dir_spare = tempdir()?;
    let temp_dir_spare = temp_dir_spare.path();
    let (boot_slot, system_slot) = if entry == inner.entry_a {
        (inner.boot_a, inner.system_a)
    } else if entry == inner.entry_b {
        (inner.boot_b, inner.system_b)
    } else {
        bail!("unknown entry");
    };
    let boot_slot = &system.slots()[boot_slot];
    let _system_slot = &system.slots()[system_slot];
    let SlotKind::Block(boot_raw) = boot_slot.kind();
    let _mounted_boot = Mounted::mount(boot_raw.device(), temp_dir_spare)?;
    let Some(root) = &system.root else {
        bail!("no parent block device");
    };
    let Some(table) = &root.table else {
        bail!("no partition table");
    };
    let root = if table.is_mbr() {
        let disk_id = get_disk_id(&root.device)?;
        if entry == inner.entry_a {
            format!("PARTUUID={disk_id}-05")
        } else {
            format!("PARTUUID={disk_id}-06")
        }
    } else {
        todo!("use the GPT partition UUID");
    };
    rpi_patch_boot(temp_dir_spare, root)?;
    Ok(())
}

#[derive(Debug)]
struct GrubEfi {
    inner: RugpiBootFlow,
}

impl BootFlow for GrubEfi {
    fn set_try_next(&self, system: &System, entry: BootGroupIdx) -> Anyhow<()> {
        if entry != self.get_default(system)? {
            grub::set_spare_flag(system)?;
        } else {
            grub::clear_spare_flag(system)?;
        }
        Ok(())
    }

    fn get_default(&self, system: &System) -> Anyhow<BootGroupIdx> {
        let config_partition = system.require_config_partition()?;
        let bootpart_env = load_grub_env(config_partition.path().join("rugpi/primary.grubenv"))?;
        let Some(bootpart) = bootpart_env.get(RUGPI_BOOTPART) else {
            bail!("Invalid bootpart environment.");
        };
        if bootpart == "2" {
            Ok(self.inner.entry_a)
        } else if bootpart == "3" {
            Ok(self.inner.entry_b)
        } else {
            bail!("Invalid default `bootpart`.");
        }
    }

    fn commit(&self, system: &System) -> Anyhow<()> {
        let mut envblk = HashMap::new();
        if system.active_boot_entry() == Some(self.inner.entry_a) {
            envblk.insert(RUGPI_BOOTPART.to_owned(), "2".to_owned());
        } else if system.active_boot_entry() == Some(self.inner.entry_b) {
            envblk.insert(RUGPI_BOOTPART.to_owned(), "3".to_owned());
        } else {
            panic!("should never happen");
        };
        let config_partition = system.require_config_partition()?;
        config_partition.ensure_writable(|| {
            write_with_hash(
                &envblk,
                &config_partition.path().join("rugpi/secondary.grubenv"),
                "/rugpi/secondary.grubenv",
            )?;
            write_with_hash(
                &envblk,
                &config_partition.path().join("rugpi/primary.grubenv"),
                "/rugpi/primary.grubenv",
            )?;
            Ok(())
        })?
    }

    fn post_install(&self, system: &System, entry: BootGroupIdx) -> Anyhow<()> {
        let temp_dir_spare = tempdir()?;
        let temp_dir_spare = temp_dir_spare.path();
        let (boot_slot, system_slot) = if entry == self.inner.entry_a {
            (self.inner.boot_a, self.inner.system_a)
        } else if entry == self.inner.entry_b {
            (self.inner.boot_b, self.inner.system_b)
        } else {
            bail!("unknown entry");
        };
        let boot_slot = &system.slots()[boot_slot];
        let _system_slot = &system.slots()[system_slot];
        let SlotKind::Block(boot_raw) = boot_slot.kind();
        let _mounted_boot = Mounted::mount(boot_raw.device(), temp_dir_spare)?;
        let Some(table) = system.root.as_ref().and_then(|root| root.table.as_ref()) else {
            bail!("no partition table");
        };
        let root_part = if entry == self.inner.entry_a {
            &table.partitions[3]
        } else if entry == self.inner.entry_b {
            &table.partitions[4]
        } else {
            panic!("should not happen");
        };
        let part_uuid = root_part
            .gpt_id
            .unwrap()
            .to_hex_str(ascii_numbers::Case::Lower);
        grub_patch_env(temp_dir_spare, part_uuid)?;
        Ok(())
    }

    fn name(&self) -> &str {
        "grub-efi"
    }
}
