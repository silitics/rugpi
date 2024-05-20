//! Utilities for repartitioning disks.

use anyhow::bail;
use serde::Deserialize;

use super::{PartitionTable, PartitionTableType};
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
pub fn repart(table: &PartitionTable, schema: &PartitionSchema) -> Anyhow<Option<PartitionTable>> {
    if table.ty() != schema.ty {
        bail!(
            "partition table types do not match ({} != {})",
            table.ty(),
            schema.ty
        );
    }
    let default_partition_ty = match schema.ty {
        PartitionTableType::Gpt => gpt_types::LINUX,
        PartitionTableType::Mbr => mbr_types::LINUX,
    };
    let align = NumBlocks::from_raw(2048);
    let mut new_table = table.clone();
    let mut next_start = table.first_usable_block().ceil_align_to(align);
    let mut last_free = table.last_usable_block().floor_align_to(align);
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
        let old = table.partitions.get(idx);
        let old_next = table.partitions.get(idx + 1);
        let ty = partition.ty.unwrap_or(default_partition_ty);
        // Compute the requested size of the partition.
        let mut size = partition.size.map(|size| table.bytes_to_blocks(size));
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
        Ok(Some(new_table))
    } else {
        Ok(None)
    }
}
