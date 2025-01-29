//! Functionality for dividing byte streams into blocks.

use byte_calc::{ByteLen, NumBytes};

pub mod casync;

/// Trait for chunking byte streams into blocks.
pub trait Chunker {
    /// Scan for the next block boundary and return it.
    ///
    /// The function returns an offset of the chunk into the provided slice.
    fn scan(&mut self, bytes: &[u8]) -> Option<usize>;

    /// Iterator over the chunks of a byte slice.
    fn chunks(mut self, mut bytes: &[u8]) -> impl Iterator<Item = &[u8]>
    where
        // We put this bound here to make the trait dyn-compatible.
        Self: Sized,
    {
        std::iter::from_fn(move || {
            if bytes.is_empty() {
                None
            } else if let Some(offset) = self.scan(bytes) {
                let chunk = &bytes[..offset];
                bytes = &bytes[offset..];
                Some(chunk)
            } else {
                let chunk = bytes;
                bytes = &[];
                Some(chunk)
            }
        })
    }
}

/// [`Chunker`] that never chunks.
#[derive(Debug, Clone, Copy)]
pub struct NeverChunker;

impl Chunker for NeverChunker {
    fn scan(&mut self, _: &[u8]) -> Option<usize> {
        None
    }
}

/// [`Chunker`] for fixed size blocks.
#[derive(Debug, Clone)]
pub struct FixedSizeChunker {
    /// Block size.
    block_size: NumBytes,
    /// Remaining bytes of the current block.
    remaining: NumBytes,
}

impl FixedSizeChunker {
    /// Create a new fixed size chunker.
    pub fn new(block_size: NumBytes) -> Self {
        assert_ne!(block_size, 0, "block size must not be zero");
        Self {
            block_size,
            remaining: block_size,
        }
    }
}

impl Chunker for FixedSizeChunker {
    fn scan(&mut self, bytes: &[u8]) -> Option<usize> {
        let take = self.remaining.min(bytes.byte_len());
        self.remaining -= take;
        if self.remaining == 0 {
            self.remaining = self.block_size;
            Some(take.raw as usize)
        } else {
            None
        }
    }
}
