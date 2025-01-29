//! Implementation of the block encoding for Rugix's update bundles.

use std::io::{BufRead, BufReader, Seek, Write};
use std::path::Path;

use block_index::compute_block_index;
use block_table::BlockTable;
use byte_calc::{ByteLen, NumBytes};
use reportify::{bail, ResultExt};
use rugix_compression::ByteProcessor;

use crate::format::Bytes;
use crate::manifest::{self, BlockEncoding};
use crate::{format, BundleResult};

pub mod block_index;
pub mod block_table;

/// Encode a payload file.
pub fn encode_payload_file(
    block_encoding: &BlockEncoding,
    payload_file: &Path,
    payload_data: &Path,
) -> BundleResult<format::BlockEncoding> {
    let block_index = compute_block_index(block_encoding, payload_file)?;
    let mut block_table = BlockTable::new();
    let mut block_sizes = Vec::new();
    let mut payload_file = BufReader::with_capacity(
        16 * 1024,
        std::fs::File::open(payload_file).whatever("unable to open payload fil")?,
    );
    let mut payload_data =
        std::fs::File::create(payload_data).whatever("unable to create payload data file")?;
    let deduplicate = block_encoding.deduplicate.unwrap_or(false);
    for block in block_index.iter() {
        if !deduplicate || block_table.insert(&block_index, block) {
            let entry = block_index.entry(block);
            payload_file
                .seek(std::io::SeekFrom::Start(entry.offset.raw))
                .whatever("unable to seek in payload file")?;
            match &block_encoding.compression {
                Some(manifest::Compression::Xz(compression)) => {
                    let mut compressor =
                        rugix_compression::XzEncoder::new(compression.level.unwrap_or(6));
                    let start_position = payload_data
                        .stream_position()
                        .whatever("unable to get position in payload data")?;
                    let mut remaining = entry.size;
                    while remaining > 0 {
                        let buffer = payload_file
                            .fill_buf()
                            .whatever("unable to read payload file")?;
                        if buffer.is_empty() {
                            bail!("payload file has been truncated");
                        };
                        let chunk = &buffer[..remaining.min(buffer.byte_len()).unwrap_usize()];
                        compressor
                            .process(chunk, &mut payload_data)
                            .whatever("unable to write compressed data")?;
                        remaining -= chunk.byte_len();
                        let consumed = chunk.len();
                        payload_file.consume(consumed);
                    }
                    compressor
                        .finalize(&mut payload_data)
                        .whatever("unable to write compressed data")?;
                    let block_size = payload_data
                        .stream_position()
                        .whatever("unable to get position in payload data")?
                        - start_position;
                    block_sizes.push(NumBytes::new(block_size));
                }
                None => {
                    let mut remaining = entry.size;
                    while remaining > 0 {
                        let buffer = payload_file
                            .fill_buf()
                            .whatever("unable to read payload file")?;
                        if buffer.is_empty() {
                            bail!("payload file has been truncated");
                        };
                        let chunk = &buffer[..remaining.min(buffer.byte_len()).unwrap_usize()];
                        payload_data
                            .write_all(&chunk)
                            .whatever("unable to write payload data")?;
                        remaining -= chunk.byte_len();
                        let consumed = chunk.len();
                        payload_file.consume(consumed);
                    }
                    block_sizes.push(entry.size);
                }
            }
        }
    }
    let is_fixed_size_chunker = match &block_index.config().chunker {
        manifest::BlockChunker::Fixed(..) => true,
        manifest::BlockChunker::Casync(..) => false,
    };
    let is_compressed = block_encoding.compression.is_some();
    let include_sizes = !is_fixed_size_chunker || is_compressed;
    Ok(format::BlockEncoding {
        block_index: Bytes {
            raw: compress_bytes(block_encoding, &block_index.into_hashes_vec()),
        },
        block_sizes: if include_sizes {
            let mut encoded_sizes = Vec::new();
            for size in block_sizes {
                encoded_sizes.extend_from_slice(
                    &u32::try_from(size.raw)
                        .expect("blocks should not be larger than 4GiB")
                        .to_be_bytes(),
                );
            }
            Some(Bytes {
                raw: compress_bytes(block_encoding, &encoded_sizes),
            })
        } else {
            None
        },
    })
}

fn compress_bytes(block_encoding: &BlockEncoding, bytes: &[u8]) -> Vec<u8> {
    match &block_encoding.compression {
        Some(manifest::Compression::Xz(compression)) => {
            let mut compressor = rugix_compression::XzEncoder::new(compression.level.unwrap_or(6));
            let mut output = Vec::new();
            compressor.process(bytes, &mut output).unwrap();
            compressor.finalize(&mut output).unwrap();
            output
        }
        None => bytes.to_vec(),
    }
}
