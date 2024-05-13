//! Utilities for working with disks, disk images, and disk streams.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use self::gpt::{Guid, GPT_TABLE_BLOCKS, GUID_STRING_LENGTH};
use crate::{
    utils::{
        ascii_numbers::parse_ascii_decimal_digit,
        units::{NumBytes, Quantity, Unit},
    },
    Anyhow,
};

pub mod blkdev;
pub mod blkpg;
pub mod gpt;
pub mod mbr;
pub mod repart;
pub mod stream;

mod sfdisk;

/// Default size of blocks.
const DEFAULT_BLOCK_SIZE: u64 = 512;

/// Unique identifier of a disk.
///
/// The disk ID also includes the type of the partition table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiskId {
    /// Unique identifier of an MBR disk.
    Mbr(mbr::MbrId),
    /// Unique identifier of a GPT disk.
    Gpt(gpt::Guid),
}

impl std::fmt::Display for DiskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskId::Mbr(id) => id.fmt(f),
            DiskId::Gpt(id) => id.fmt(f),
        }
    }
}

/// Partition table.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PartitionTable {
    /// Disk ID.
    pub disk_id: DiskId,
    /// Size of the disk.
    pub disk_size: NumBlocks,
    /// Block size of the disk.
    pub block_size: NumBytes,
    /// Partitions of the disk.
    pub partitions: Vec<Partition>,
}

impl PartitionTable {
    /// Create an empty partition table with the given ID.
    pub fn new(id: DiskId, size: NumBlocks) -> Self {
        Self {
            disk_id: id,
            disk_size: size,
            block_size: NumBytes::from_value(DEFAULT_BLOCK_SIZE),
            partitions: Vec::new(),
        }
    }

    /// Read the partition table from a device or image.
    pub fn read(dev: impl AsRef<Path>) -> Anyhow<Self> {
        sfdisk::sfdisk_read(dev.as_ref())
    }

    /// The size of the disk in bytes.
    pub fn size(&self) -> NumBytes {
        NumBytes::from_value(self.block_size.into_value() * self.disk_size.into_value())
    }

    /// The type of the partition table.
    pub fn ty(&self) -> PartitionTableType {
        match self.disk_id {
            DiskId::Mbr(_) => PartitionTableType::Mbr,
            DiskId::Gpt(_) => PartitionTableType::Gpt,
        }
    }

    /// Returns whether the table is a GUID partition table.
    pub fn is_gpt(&self) -> bool {
        matches!(self.disk_id, DiskId::Gpt(_))
    }

    /// Returns whether the table is an MBR partition table.
    pub fn is_mbr(&self) -> bool {
        matches!(self.disk_id, DiskId::Mbr(_))
    }

    /// Convert blocks to bytes.
    pub fn blocks_to_bytes(&self, blocks: NumBlocks) -> NumBytes {
        NumBytes::from_value(blocks.into_value() * self.block_size.into_value())
    }

    /// Convert bytes to blocks.
    pub fn bytes_to_blocks(&self, bytes: NumBytes) -> NumBlocks {
        NumBlocks::from_value(bytes.into_value().div_ceil(self.block_size.into_value()))
    }

    /// The first usable block.
    pub fn first_usable_block(&self) -> NumBlocks {
        GPT_TABLE_BLOCKS + NumBlocks::ONE
    }

    /// The last usable block.
    pub fn last_usable_block(&self) -> NumBlocks {
        self.disk_size - GPT_TABLE_BLOCKS - NumBlocks::ONE
    }

    /// Write the partition table to a device or image.
    pub fn write(&self, dev: impl AsRef<Path>) -> Anyhow<()> {
        sfdisk::sfdisk_write(self, dev.as_ref())
    }

    /// Check invariants of the partition table.
    ///
    /// - Partitions should be sorted by their number.
    /// - Partitions should be sorted by their first block.
    /// - Partitions should be within the bounds of the disk.
    pub fn verify(&self) {
        let mut next_free = self.first_usable_block();
        let mut last_usable = self.last_usable_block();
        let mut next_number = 0;
        for partition in &self.partitions {
            if partition.start < next_free {
                panic!("invalid");
            }
            if partition.number < next_number {
                panic!("invalid");
            }
            next_number = partition.number + 1;
            next_free = partition.start + partition.size;
            if partition.ty.is_extended() {
                next_free = partition.start + NumBlocks::from_value(63);
                last_usable = partition.start + partition.size;
            }
        }
        if next_free > last_usable {
            panic!("invalid");
        }
    }
}

/// Partition of a disk.
#[derive(Clone, Debug)]
pub struct Partition {
    /// Number of the partition.
    pub number: u8,
    /// Start sector of the partition.
    pub start: NumBlocks,
    /// Size of the partition.
    pub size: NumBlocks,
    /// Type of the partition.
    pub ty: PartitionType,
    /// Optional name of the partition.
    pub name: Option<String>,
    /// Optional unique identifier of the partition.
    pub gpt_id: Option<gpt::Guid>,
}

impl Partition {
    /// The end of the partition.
    pub fn end(&self) -> NumBlocks {
        self.start + self.size
    }
}

/// Partition type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionType {
    /// MBR partition type.
    Mbr(u8),
    /// GPT partition type.
    Gpt(gpt::Guid),
}

impl PartitionType {
    pub fn is_free(&self) -> bool {
        match self {
            PartitionType::Mbr(ty) => *ty == 0x00,
            PartitionType::Gpt(guid) => guid.is_zero(),
        }
    }

    pub fn is_extended(&self) -> bool {
        match self {
            PartitionType::Mbr(id) => matches!(id, 0x05 | 0x0F),
            PartitionType::Gpt(_) => false,
        }
    }
}

impl<'de> serde::Deserialize<'de> for PartitionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        if string.len() == GUID_STRING_LENGTH {
            Ok(Self::Gpt(Guid::from_hex_str(&string).map_err(|_| {
                <D::Error as serde::de::Error>::invalid_value(
                    serde::de::Unexpected::Str(&string),
                    &"a partition type",
                )
            })?))
        } else {
            Ok(Self::Mbr(u8::from_str_radix(&string, 16).map_err(
                |_| {
                    <D::Error as serde::de::Error>::invalid_value(
                        serde::de::Unexpected::Str(&string),
                        &"a partition type",
                    )
                },
            )?))
        }
    }
}

impl std::fmt::Display for PartitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartitionType::Mbr(id) => f.write_fmt(format_args!("{:02x}", *id)),
            PartitionType::Gpt(id) => id.fmt(f),
        }
    }
}

impl std::fmt::Debug for PartitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mbr(id) => f.write_fmt(format_args!("Mbr(0x{id:02x})")),
            Self::Gpt(id) => f.write_fmt(format_args!("Gpt({id})")),
        }
    }
}

/// Number of blocks unit.
pub struct NumBlocksUnit(());

impl Unit for NumBlocksUnit {
    fn name() -> &'static str {
        "NumBlocks"
    }

    fn symbol() -> &'static str {
        "blocks"
    }
}

/// Number of blocks.
pub type NumBlocks = Quantity<u64, NumBlocksUnit>;

impl NumBlocks {
    /// One block.
    pub const ONE: NumBlocks = NumBlocks::from_value(1);

    /// Align the block number rounding downward.
    pub fn floor_align_to(self, align: Self) -> Self {
        Self::from_value(align.into_value() * (self / align))
    }

    /// Align the block number rounding upward.
    pub fn ceil_align_to(self, align: Self) -> Self {
        Self::from_value(align.into_value() * (self.into_value().div_ceil(align.into_value())))
    }
}

/// Convert a size string to bytes.
pub const fn parse_size(size: &str) -> Result<NumBytes, InvalidSize> {
    let size = size.as_bytes();
    if size.is_empty() {
        return Ok(NumBytes::from_value(0));
    }
    let mut last = size.len() - 1;
    let factor: u64 = match size[last] {
        b'K' => 1 << 10,
        b'M' => 1 << 20,
        b'G' => 1 << 30,
        b'T' => 1 << 40,
        _ => 1,
    };
    if factor != 1 {
        last -= 1;
    }
    while last > 0 && size[last] == b' ' {
        last -= 1;
    }
    let mut pos = 0;
    let mut value = 0;
    while pos <= last {
        if size[pos] != b'_' {
            value *= 10;
            value += match parse_ascii_decimal_digit(size[pos], pos) {
                Ok(digit) => digit as u64,
                Err(_) => return Err(InvalidSize { pos }),
            };
        }
        pos += 1;
    }
    Ok(NumBytes::from_value(value * factor))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PartitionTableType {
    Gpt,
    Mbr,
}

impl std::fmt::Display for PartitionTableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartitionTableType::Gpt => f.write_str("gpt"),
            PartitionTableType::Mbr => f.write_str("mbr"),
        }
    }
}

/// Error indicating an invalid size.
#[derive(Debug, Clone, Error)]
#[error("invalid character at position {pos}")]
pub struct InvalidSize {
    pos: usize,
}

impl<'de> serde::Deserialize<'de> for NumBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        parse_size(&string).map_err(|_| {
            <D::Error as serde::de::Error>::invalid_value(
                serde::de::Unexpected::Str(&string),
                &"a size",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_parse_size() {
        assert_eq!(
            parse_size("512M").unwrap(),
            NumBytes::from_value(512 * (1 << 20))
        );
    }

    #[test]
    pub fn test_block_alignment() {
        assert_eq!(
            NumBlocks::from_value(2048).ceil_align_to(NumBlocks::from_value(2048)),
            NumBlocks::from_value(2048)
        );
        assert_eq!(
            NumBlocks::from_value(2048).floor_align_to(NumBlocks::from_value(2048)),
            NumBlocks::from_value(2048)
        );
        assert_eq!(
            NumBlocks::from_value(2049).ceil_align_to(NumBlocks::from_value(2048)),
            NumBlocks::from_value(4096)
        );
        assert_eq!(
            NumBlocks::from_value(2049).floor_align_to(NumBlocks::from_value(2048)),
            NumBlocks::from_value(2048)
        )
    }
}
