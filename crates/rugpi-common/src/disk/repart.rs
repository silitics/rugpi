//! Utilities for repartitioning disks.

use anyhow::bail;
use serde::Deserialize;

use super::{parse_size, PartitionTable, PartitionTableType};
use crate::{
    disk::{gpt::gpt_types, mbr::mbr_types, NumBlocks, Partition, PartitionType},
    utils::units::NumBytes,
    Anyhow,
};

/// Partition schema.
#[derive(Debug, Clone, Deserialize)]
pub struct PartitionSchema {
    #[serde(rename = "type")]
    pub ty: PartitionTableType,
    pub partitions: Vec<SchemaPartition>,
}

/// Partition of a schema.
#[derive(Debug, Clone, Deserialize)]
pub struct SchemaPartition {
    pub number: Option<u8>,
    pub name: Option<String>,
    pub size: Option<NumBytes>,
    #[serde(rename = "type")]
    pub ty: Option<PartitionType>,
}

/// Repartition the given table based on the provided schema.
///
/// Currently, the algorithm is very simple and matches up partitions based on their
/// index in the schema and partition table.
pub fn repart(
    old_table: &PartitionTable,
    schema: &PartitionSchema,
) -> Anyhow<Option<PartitionTable>> {
    if old_table.ty() != schema.ty {
        bail!(
            "partition table types do not match ({} != {})",
            old_table.ty(),
            schema.ty
        );
    }
    let default_partition_ty = match schema.ty {
        PartitionTableType::Gpt => gpt_types::LINUX,
        PartitionTableType::Mbr => mbr_types::LINUX,
    };
    let align = NumBlocks::from_raw(2048);
    let mut new_table = old_table.clone();
    let mut next_start = old_table.first_usable_block().ceil_align_to(align);
    let mut last_free = old_table.last_usable_block().floor_align_to(align);
    let mut in_extended = false;
    let mut has_changed = false;
    for (idx, partition) in schema.partitions.iter().enumerate() {
        println!(
            "Partition: {}, Next Start: {next_start}, Last Free: {last_free}",
            idx + 1
        );
        if in_extended {
            next_start = (next_start + NumBlocks::from_raw(63)).ceil_align_to(align);
        }
        let old = old_table.partitions.get(idx);
        let old_next = old_table.partitions.get(idx + 1);
        let ty = partition.ty.unwrap_or(default_partition_ty);
        // Compute the requested size of the partition.
        let mut size = partition.size.map(|size| old_table.bytes_to_blocks(size));
        if let Some(old) = old {
            next_start = old.start;
            if old.ty != ty {
                bail!(
                    "partition types of partition {} do not match ({} != {})",
                    idx + 1,
                    old.ty,
                    ty
                )
            }
            size = size.map(|size| size.max(old.size));
        } else {
            next_start = next_start.ceil_align_to(align);
        }
        let available = if ty.is_extended() {
            last_free - next_start
        } else if let Some(old_next) = old_next {
            let old_start = old.expect("old partition must exist").start;
            old_next.start - old_start
        } else {
            if next_start >= last_free {
                println!("{new_table:#?}");
                bail!("insufficient space, cannot add partition {}", idx + 1);
            }
            last_free - next_start
        };
        let size = size.unwrap_or(available).min(available);
        println!("  Available: {available}, Size: {size}");
        if let Some(new) = new_table.partitions.get_mut(idx) {
            if new.size != size {
                has_changed = true;
            }
            new.size = size;
        } else {
            has_changed = true;
            new_table.partitions.push(Partition {
                number: (idx + 1) as u8,
                start: next_start,
                size,
                ty,
                name: None,
                gpt_id: None,
            })
        }
        if ty.is_extended() {
            last_free = next_start + size;
            in_extended = true;
        } else {
            next_start += size;
        }
    }
    if has_changed {
        check_new_table(old_table, &new_table)?;
        Ok(Some(new_table))
    } else {
        Ok(None)
    }
}

/// Perform a sanity check of the new partition table.
///
/// The conditions checked here should always be ensured by the repartitioning algorithm.
/// Nevertheless, we check it them here explicitly to catch bugs that otherwise would mess
/// up the partition table leading to potential data loss.
///
/// Arguably the checks here should panic as they correspond to internal invariants. We
/// return errors instead such that they are handled gracefully.
fn check_new_table(old_table: &PartitionTable, new_table: &PartitionTable) -> Anyhow<()> {
    // We first validate the new table ensuring that no partitions overlap.
    new_table.validate()?;
    if old_table.disk_id != new_table.disk_id {
        bail!("BUG: Partition table id must not be changed.");
    }
    if old_table.ty() != new_table.ty() {
        bail!("BUG: Types of old and new partition table must be the same.");
    }
    if old_table.partitions.len() > new_table.partitions.len() {
        bail!("BUG: Partitions must not be deleted.");
    }
    for (old, new) in old_table.partitions.iter().zip(new_table.partitions.iter()) {
        if old.ty != new.ty {
            bail!("BUG: Types of old and new partition must be the same.");
        }
        if old.start != new.start {
            bail!("BUG: Old and new partition must start at the same offset.");
        }
        if old.size > new.size {
            bail!("BUG: New partition must not be smaller than old partition.");
        }
        if old.gpt_id != new.gpt_id {
            bail!("BUG: GPT UUID of partition must not be changed.");
        }
    }
    Ok(())
}

pub fn generic_mbr_partition_schema(system_size: NumBytes) -> PartitionSchema {
    PartitionSchema {
        ty: PartitionTableType::Mbr,
        partitions: vec![
            SchemaPartition {
                number: None,
                name: None,
                size: Some(parse_size("256M").unwrap()),
                ty: Some(mbr_types::FAT32_LBA),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(parse_size("128M").unwrap()),
                ty: Some(mbr_types::FAT32_LBA),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(parse_size("128M").unwrap()),
                ty: Some(mbr_types::FAT32_LBA),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: None,
                ty: Some(mbr_types::EXTENDED),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(system_size),
                ty: Some(mbr_types::LINUX),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(system_size),
                ty: Some(mbr_types::LINUX),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: None,
                ty: Some(mbr_types::LINUX),
            },
        ],
    }
}

pub fn generic_efi_partition_schema(system_size: NumBytes) -> PartitionSchema {
    PartitionSchema {
        ty: PartitionTableType::Gpt,
        partitions: vec![
            SchemaPartition {
                number: None,
                name: None,
                size: Some(parse_size("256M").unwrap()),
                ty: Some(gpt_types::EFI),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(parse_size("256M").unwrap()),
                ty: Some(gpt_types::LINUX),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(parse_size("256M").unwrap()),
                ty: Some(gpt_types::LINUX),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(system_size),
                ty: Some(gpt_types::LINUX),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: Some(system_size),
                ty: Some(gpt_types::LINUX),
            },
            SchemaPartition {
                number: None,
                name: None,
                size: None,
                ty: Some(gpt_types::LINUX),
            },
        ],
    }
}
