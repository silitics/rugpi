use std::path::Path;

use byte_calc::NumBytes;

/// Provider for stored blocks.
pub trait StoredBlockProvider {
    /// Query the provider for a block with the given hash.
    fn query(&self, hash: &[u8]) -> Option<StoredBlock<'_>>;

    fn has_stored_blocks(&self) -> bool;
}

/// Stored block.
#[derive(Debug, Clone, Copy)]
pub struct StoredBlock<'provider> {
    /// File containing the block.
    pub file: &'provider Path,
    /// Offset of the block in the file.
    pub offset: NumBytes,
    /// Size of the block in the file.
    pub size: NumBytes,
}
