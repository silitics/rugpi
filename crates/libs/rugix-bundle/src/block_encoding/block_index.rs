//! Provides the [`BlockIndex`] data structure.

use std::borrow::Cow;
use std::io::{BufRead, BufReader};
use std::path::Path;

use byte_calc::{ByteLen, NumBytes};

use reportify::ResultExt;
use rugix_chunker::{AnyChunker, Chunker, ChunkerAlgorithm};
use rugix_hashes::{HashAlgorithm, Hasher};

use crate::manifest::BlockEncoding;
use crate::BundleResult;

pub struct RawBlockIndex<'hashes> {
    hashes: Cow<'hashes, [u8]>,
    hash_algorithm: HashAlgorithm,
}

impl<'hashes> RawBlockIndex<'hashes> {
    pub fn new(hashes: &'hashes [u8], hash_algorithm: HashAlgorithm) -> Self {
        Self {
            hashes: Cow::Borrowed(hashes),
            hash_algorithm,
        }
    }

    pub fn block_hash(&self, block: BlockId) -> &[u8] {
        let start = block.raw * self.hash_algorithm.hash_size();
        let end = (block.raw + 1) * self.hash_algorithm.hash_size();
        &self.hashes[start..end]
    }
}

/// Build a block index for the provided payload file.
pub fn compute_block_index(
    block_encoding: &BlockEncoding,
    payload_file: &Path,
) -> BundleResult<BlockIndex> {
    let index_config = BlockIndexConfig {
        hash_algorithm: block_encoding
            .hash_algorithm
            .unwrap_or(rugix_hashes::HashAlgorithm::Sha512_256),
        chunker: block_encoding.chunker.clone(),
    };
    let mut index_builder = BlockIndexBuilder::new(index_config.clone())?;
    let mut payload_file =
        BufReader::new(std::fs::File::open(payload_file).whatever("unable to open payload file")?);
    Ok(loop {
        let buffer = payload_file
            .fill_buf()
            .whatever("unable to read payload file")?;
        if buffer.is_empty() {
            break index_builder.finalize();
        }
        index_builder.process(buffer);
        let consumed = buffer.len();
        payload_file.consume(consumed);
    })
}

/// Id of a block in a [`BlockIndex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId {
    /// Raw block number.
    pub(crate) raw: usize,
}

/// Entry of a block in a [`BlockIndex`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct BlockIndexEntry<'idx> {
    /// Raw hash of the block.
    pub hash: &'idx [u8],
    /// Offset of the block.
    pub offset: NumBytes,
    /// Size of the block.
    pub size: NumBytes,
}

/// Block index.
#[derive(Debug)]
pub struct BlockIndex {
    /// Configuration of the block index.
    config: BlockIndexConfig,
    /// Block hashes.
    hashes: Vec<u8>,
    /// Block offsets.
    offsets: Vec<NumBytes>,
    /// Block sizes.
    sizes: Vec<NumBytes>,
}

impl BlockIndex {
    /// Create an empty block index with the given configuration.
    fn new(config: BlockIndexConfig) -> Self {
        Self {
            config,
            hashes: Vec::new(),
            offsets: Vec::new(),
            sizes: Vec::new(),
        }
    }

    /// Convert the index into a raw hash vector.
    pub fn into_hashes_vec(self) -> Vec<u8> {
        self.hashes
    }

    /// Configuration of the block index.
    pub fn config(&self) -> &BlockIndexConfig {
        &self.config
    }

    /// Retrieve the raw hash of the given block.
    pub fn block_hash(&self, block: BlockId) -> &[u8] {
        let start = block.raw * self.config.hash_algorithm.hash_size();
        let end = (block.raw + 1) * self.config.hash_algorithm.hash_size();
        &self.hashes[start..end]
    }

    /// Retrieve the offset of the given block.
    pub fn block_offset(&self, block: BlockId) -> NumBytes {
        self.offsets[block.raw]
    }

    /// Retrieve the size of the given block.
    pub fn block_size(&self, block: BlockId) -> NumBytes {
        self.sizes[block.raw]
    }

    /// Retrieve the entry for the given block.
    pub fn entry(&self, block: BlockId) -> BlockIndexEntry {
        BlockIndexEntry {
            hash: self.block_hash(block),
            offset: self.block_offset(block),
            size: self.block_size(block),
        }
    }

    /// Iterate over the block ids.
    pub fn iter(&self) -> impl Iterator<Item = BlockId> {
        (0..self.offsets.len()).map(|idx| BlockId { raw: idx })
    }

    /// Push a new entry into the index.
    ///
    /// # Panics
    ///
    /// Panics when the hash size of the entry does not match the hash size of the index's
    /// hash algorithm.
    fn push(&mut self, entry: BlockIndexEntry) -> BlockId {
        assert_eq!(
            entry.hash.len(),
            self.config.hash_algorithm.hash_size(),
            "invalid hash size in entry"
        );
        let idx = BlockId {
            raw: self.offsets.len(),
        };
        self.hashes.extend_from_slice(entry.hash);
        self.offsets.push(entry.offset);
        self.sizes.push(entry.size);
        idx
    }
}

/// Block index configuration.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BlockIndexConfig {
    /// Hash algorithm to use.
    pub hash_algorithm: HashAlgorithm,
    /// Chunker configuration.
    pub chunker: ChunkerAlgorithm,
}

/// Block index builder.
#[derive(Debug)]
pub struct BlockIndexBuilder {
    /// Hasher for the computing the hash of the pending block.
    hasher: Hasher,
    /// Chunker to determine the block boundaries.
    chunker: AnyChunker,
    /// Block index being built.
    index: BlockIndex,
    /// Offset of the pending block in the byte stream.
    pending_block_offset: NumBytes,
    /// Current size of the pending block in the byte stream.
    pending_block_size: NumBytes,
}

impl BlockIndexBuilder {
    /// Create a new block index builder with the given configuration.
    pub fn new(config: BlockIndexConfig) -> BundleResult<Self> {
        Ok(Self {
            hasher: config.hash_algorithm.hasher(),
            chunker: config
                .chunker
                .chunker()
                .whatever("unable to create chunker")?,
            index: BlockIndex::new(config),
            pending_block_offset: NumBytes::ZERO,
            pending_block_size: NumBytes::ZERO,
        })
    }

    /// Process the provided input.
    pub fn process(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            let boundary = self.chunker.scan(input);
            let offset = boundary.unwrap_or(input.len());
            let chunk = &input[..offset];
            self.hasher.update(chunk);
            self.pending_block_size += chunk.byte_len();
            if boundary.is_some() {
                self.finalize_block();
            }
            input = &input[offset..];
        }
    }

    /// Finalize the and return the index.
    pub fn finalize(mut self) -> BlockIndex {
        if self.pending_block_size > 0 {
            self.finalize_block();
        }
        self.index
    }

    /// Finalize the current block.
    fn finalize_block(&mut self) {
        let hasher = self.index.config.hash_algorithm.hasher();
        let hash = std::mem::replace(&mut self.hasher, hasher).finalize();
        let entry = BlockIndexEntry {
            hash: hash.raw(),
            offset: self.pending_block_offset,
            size: self.pending_block_size,
        };
        self.index.push(entry);
        self.pending_block_offset += self.pending_block_size;
        self.pending_block_size = NumBytes::ZERO;
    }
}
