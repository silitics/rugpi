//! Provides the [`BlockTable`] data structure.

use std::hash::BuildHasher;

use hashbrown::{hash_table, DefaultHashBuilder, HashTable};

use super::block_index::{BlockId, BlockIndex, RawBlockIndex};

/// Block table.
#[derive(Debug)]
pub struct BlockTable {
    /// Block hash table.
    table: HashTable<BlockId>,
    /// Hasher for the hash table.
    hasher: DefaultHashBuilder,
}

impl BlockTable {
    /// Create an empty block table.
    pub fn new() -> Self {
        Self {
            table: HashTable::new(),
            hasher: DefaultHashBuilder::default(),
        }
    }

    /// Create a block table from the provided index.
    pub fn from_index(index: &BlockIndex) -> Self {
        let mut table = Self::new();
        for block in index.iter() {
            table.insert(index, block);
        }
        table
    }

    pub fn get_raw(&self, index: &RawBlockIndex, hash: &[u8]) -> Option<BlockId> {
        let block_hash = self.hasher.hash_one(hash);
        self.table
            .find(block_hash, |other| hash == index.block_hash(*other))
            .cloned()
    }

    pub fn insert_raw(&mut self, index: &RawBlockIndex, block: BlockId) -> bool {
        let block_hash = self.hasher.hash_one(index.block_hash(block));
        match self.table.entry(
            block_hash,
            |other| index.block_hash(block) == index.block_hash(*other),
            |other| self.hasher.hash_one(index.block_hash(*other)),
        ) {
            hash_table::Entry::Occupied(_) => false,
            hash_table::Entry::Vacant(entry) => {
                entry.insert(block);
                true
            }
        }
    }

    /// Insert a block into the table.
    pub fn insert(&mut self, index: &BlockIndex, block: BlockId) -> bool {
        let block_hash = self.hasher.hash_one(index.block_hash(block));
        match self.table.entry(
            block_hash,
            |other| index.block_hash(block) == index.block_hash(*other),
            |other| self.hasher.hash_one(index.block_hash(*other)),
        ) {
            hash_table::Entry::Occupied(_) => false,
            hash_table::Entry::Vacant(entry) => {
                entry.insert(block);
                true
            }
        }
    }
}
