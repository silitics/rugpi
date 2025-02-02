//! Utilities for working with MBR partition tables.

/// MBR disk id.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MbrId {
    id: u32,
}

impl MbrId {
    /// Create an MBR disk id.
    pub const fn new(id: u32) -> Self {
        Self { id }
    }

    pub const fn into_raw(self) -> u32 {
        self.id
    }
}

impl std::fmt::Display for MbrId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("0x{:08x}", self.id))
    }
}

impl std::fmt::Debug for MbrId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("MbrId(0x{:08x})", self.id))
    }
}

/// MBR partition types.
pub mod mbr_types {
    use crate::disk::PartitionType;

    /// Extended partition with CHS addressing.
    pub const EXTENDED_CHS: PartitionType = PartitionType::Mbr(0x05);
    /// Extended partition with LBA addressing.
    pub const EXTENDED_LBA: PartitionType = PartitionType::Mbr(0x0F);

    pub const EXTENDED: PartitionType = EXTENDED_CHS;

    /// FAT32 partition with LBA addressing.
    pub const FAT32_LBA: PartitionType = PartitionType::Mbr(0x0C);

    /// Linux filesystem.
    pub const LINUX: PartitionType = PartitionType::Mbr(0x83);
}
