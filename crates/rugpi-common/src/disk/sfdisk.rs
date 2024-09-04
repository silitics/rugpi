use std::{
    fmt::Write,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::Path,
    str::FromStr,
};

use anyhow::anyhow;
use serde::Deserialize;
use xscript::{read_str, run, Run};

use super::{
    blkdev::BlockDevice, gpt::Guid, mbr, DiskId, NumBlocks, Partition, PartitionTable,
    PartitionType,
};
use crate::{utils::units::NumBytes, Anyhow};

/// Path to the `sfdisk` executable.
const SFDISK: &str = "/usr/sbin/sfdisk";

pub(crate) fn sfdisk_read(dev: &Path) -> Anyhow<PartitionTable> {
    let json_table =
        serde_json::from_str::<SfdiskJson>(&read_str!([SFDISK, "--dump", "--json", dev])?)?
            .partition_table;
    let metadata = dev.metadata()?;
    let size = if metadata.file_type().is_block_device() {
        NumBlocks::from_raw(BlockDevice::new(dev)?.size()? / json_table.sector_size)
    } else {
        NumBlocks::from_raw(metadata.size() / json_table.sector_size)
    };
    let id = match json_table.label {
        SfdiskJsonLabel::Dos => DiskId::Mbr(
            json_table
                .id
                .get(2..)
                .and_then(|id| u32::from_str_radix(id, 16).ok().map(mbr::MbrId::new))
                .ok_or_else(|| {
                    anyhow!(
                        "invalid MBR disk id {:?} returned by `sfdisk`",
                        json_table.id
                    )
                })?,
        ),
        SfdiskJsonLabel::Gpt => DiskId::Gpt(json_table.id.parse().map_err(|_| {
            anyhow!(
                "invalid GPT disk id {:?} returned from `sfdisk`",
                json_table.id
            )
        })?),
    };
    let mut partitions = json_table
        .partitions
        .into_iter()
        .map(|partition| {
            let number = partition
                .node
                .rsplit_once(|c: char| !c.is_ascii_digit())
                .and_then(|(_, suffix)| u8::from_str(suffix).ok())
                .ok_or_else(|| {
                    anyhow!(
                        "invalid partition name {:?} returned from `sfdisk`",
                        partition.node
                    )
                })?;
            let ty = match id {
                DiskId::Mbr(_) => PartitionType::Mbr(u8::from_str_radix(&partition.ty, 16)?),
                DiskId::Gpt(_) => {
                    PartitionType::Gpt(Guid::from_hex_str(&partition.ty).map_err(|_| {
                        anyhow!(
                            "invalid GPT partition type {:?} returned from `sfdisk`",
                            partition.ty
                        )
                    })?)
                }
            };
            let gpt_id = partition
                .uuid
                .map(|guid| {
                    Guid::from_hex_str(&guid).map_err(|_| {
                        anyhow!("invalid partition GUID {:?} returned from `sfdisk`", guid)
                    })
                })
                .transpose()?;
            Ok(Partition {
                number,
                start: NumBlocks::from_raw(partition.start),
                size: NumBlocks::from_raw(partition.size),
                ty,
                name: Some(partition.node),
                gpt_id,
            })
        })
        .collect::<Anyhow<Vec<_>>>()?;
    partitions.sort_by(|x, y| x.start.cmp(&y.start));
    Ok(PartitionTable {
        disk_id: id,
        disk_size: size,
        block_size: NumBytes::from_raw(json_table.sector_size),
        partitions,
    })
}

pub(crate) fn sfdisk_write(table: &PartitionTable, dev: &Path) -> Anyhow<()> {
    let mut script = String::new();
    match table.disk_id {
        DiskId::Mbr(_) => script.push_str("label: dos\n"),
        DiskId::Gpt(_) => script.push_str("label: gpt\n"),
    }
    writeln!(&mut script, "label-id: {}", table.disk_id).unwrap();
    for partition in &table.partitions {
        write!(&mut script, "{}: ", partition.number).unwrap();
        write!(
            &mut script,
            "start={},size={},type={}",
            partition.start.into_raw(),
            partition.size.into_raw(),
            partition.ty
        )
        .unwrap();
        if let Some(gpt_id) = partition.gpt_id {
            write!(&mut script, ",uuid={}", gpt_id).unwrap();
        }
        script.push('\n');
    }

    println!("{script}");

    run!([SFDISK, "--no-reread", dev].with_stdin(script))?;
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SfdiskJson {
    #[serde(rename = "partitiontable")]
    partition_table: SfdiskJsonTable,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SfdiskJsonTable {
    label: SfdiskJsonLabel,
    id: String,
    device: String,
    unit: String,
    #[serde(rename = "sectorsize")]
    sector_size: u64,
    // This field is missing if there are no partitions.
    #[serde(default)]
    partitions: Vec<SfdiskJsonPartition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum SfdiskJsonLabel {
    Dos,
    Gpt,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SfdiskJsonPartition {
    node: String,
    start: u64,
    size: u64,
    #[serde(rename = "type")]
    ty: String,
    uuid: Option<String>,
}
