use std::fs::File;
use std::io::{Read, Seek, Write};

use block_provider::StoredBlockProvider;
use byte_calc::{ByteLen, NumBytes};
use reportify::{bail, whatever, ResultExt};
use rugix_compression::{ByteProcessor, CompressionFormat};
use rugix_hashes::HashDigest;

use crate::block_encoding::block_index::{BlockId, RawBlockIndex};
use crate::block_encoding::block_table::BlockTable;
use crate::format::decode::decode_slice;
use crate::format::stlv::{read_atom_head, skip, write_atom_head, AtomHead, Tag};
use crate::format::{self, tags};
use crate::source::BundleSource;
use crate::{BundleResult, BUNDLE_HEADER_SIZE_LIMIT, PAYLOAD_HEADER_SIZE_LIMIT};

pub mod block_provider;

pub struct BundleReader<S> {
    source: S,
    header: format::BundleHeader,
    next_payload: usize,
}

impl<S: BundleSource> BundleReader<S> {
    pub fn start(mut source: S, header_hash: Option<HashDigest>) -> BundleResult<Self> {
        let _ = expect_start(&mut source, tags::BUNDLE);
        let mut bundle_header = Vec::new();
        let header_head = skip_until_start(&mut source, tags::BUNDLE_HEADER)?;
        read_into_vec(
            &mut source,
            &mut bundle_header,
            header_head,
            BUNDLE_HEADER_SIZE_LIMIT,
        )?;
        if let Some(digest) = header_hash {
            if digest.algorithm().hash(&bundle_header) != digest {
                bail!("invalid header hash");
            }
        }
        let header = decode_slice::<format::BundleHeader>(&bundle_header)?;
        let _ = skip_until_start(&mut source, tags::PAYLOADS)?;
        Ok(Self {
            source,
            header,
            next_payload: 0,
        })
    }

    pub fn header(&self) -> &format::BundleHeader {
        &self.header
    }

    pub fn next_payload(&mut self) -> BundleResult<Option<PayloadReader<'_, S>>> {
        if self.next_payload >= self.header.payload_index.len() {
            return Ok(None);
        }
        let this_payload = self.next_payload;
        self.next_payload += 1;
        let entry = &self.header.payload_index[this_payload];
        let _ = expect_start(&mut self.source, tags::PAYLOAD);
        let header_atom = skip_until_start(&mut self.source, tags::PAYLOAD_HEADER)?;
        let mut header_bytes = Vec::new();
        read_into_vec(
            &mut self.source,
            &mut header_bytes,
            header_atom,
            PAYLOAD_HEADER_SIZE_LIMIT,
        )?;
        if self.header.hash_algorithm.hash(&header_bytes).raw() != entry.header_hash.raw {
            bail!("invalid payload header hash");
        }
        let remaining_data = skip_until_value(&mut self.source, tags::PAYLOAD_DATA)?;
        Ok(Some(PayloadReader {
            idx: this_payload,
            reader: self,
            header: decode_slice(&header_bytes)?,
            remaining_data,
        }))
    }
}

pub struct PayloadReader<'r, S> {
    idx: usize,
    reader: &'r mut BundleReader<S>,
    header: format::PayloadHeader,
    remaining_data: NumBytes,
}

impl<'r, S: BundleSource> PayloadReader<'r, S> {
    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn header(&self) -> &format::PayloadHeader {
        &self.header
    }

    pub fn entry(&self) -> &format::PayloadEntry {
        &self.reader.header().payload_index[self.idx]
    }

    pub fn skip(self) -> BundleResult<()> {
        self.reader.source.skip(self.remaining_data)?;
        skip_until_end(&mut self.reader.source, tags::PAYLOAD)?;
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> BundleResult<usize> {
        let max_chunk = buf.byte_len().min(self.remaining_data).unwrap_usize();
        let read = self.reader.source.read(&mut buf[..max_chunk])?;
        self.remaining_data -= NumBytes::from_usize(read);
        Ok(read)
    }

    pub fn decode_into<T: PayloadTarget>(
        mut self,
        mut target: T,
        provider: Option<&dyn StoredBlockProvider>,
    ) -> BundleResult<()> {
        let mut buffer = vec![0; 8192];
        let mut payload_hasher = self.reader.header.hash_algorithm.hasher();
        if let Some(block_encoding) = self.header.block_encoding {
            let mut block_index_raw = block_encoding.block_hashes.raw;
            if let Some(format) = block_encoding.compression {
                block_index_raw = uncompress_bytes(format, &block_index_raw);
            }
            let block_sizes = block_encoding.block_sizes.map(|block_sizes| {
                let mut block_sizes = block_sizes.raw;
                if let Some(format) = block_encoding.compression {
                    block_sizes = uncompress_bytes(format, &block_sizes);
                }
                let mut decoded_sizes = Vec::new();
                for chunk in block_sizes.chunks_exact(4) {
                    decoded_sizes.push(u32::from_be_bytes(chunk.try_into().unwrap()))
                }
                decoded_sizes
            });
            let fixed_block_size = match block_encoding.chunker {
                rugix_chunker::ChunkerAlgorithm::Casync { .. } => None,
                rugix_chunker::ChunkerAlgorithm::Fixed { block_size_kib } => {
                    Some((block_size_kib as u32) * 1024)
                }
            };
            if fixed_block_size.is_none() && block_sizes.is_none() {
                bail!("variable-size index needs block sizes")
            }
            let raw_index = RawBlockIndex::new(&block_index_raw, block_encoding.hash_algorithm);
            let mut table = BlockTable::new();
            let mut current_target_offset = NumBytes::ZERO;
            let num_blocks = block_index_raw.len() / block_encoding.hash_algorithm.hash_size();
            let mut target_offsets = Vec::with_capacity(num_blocks);
            let mut target_sizes = Vec::with_capacity(num_blocks);
            let mut next_size_idx = 0;
            for (idx, block_hash) in block_index_raw
                .chunks_exact(block_encoding.hash_algorithm.hash_size())
                .enumerate()
            {
                let block_id = BlockId { raw: idx };
                let is_fresh = table.insert_raw(&raw_index, block_id);
                let first_idx = table.get_raw(&raw_index, block_hash).unwrap();
                // Get the data, afterwards buffer should contain the uncompressed block.
                if is_fresh || !block_encoding.deduplicated {
                    // We need to read the block from the source.
                    // Determine the size of the block in the encoding.
                    let block_size = (block_sizes
                        .as_ref()
                        .map(|sizes| sizes[next_size_idx])
                        .or(fixed_block_size)
                        .unwrap() as u64)
                        .min(self.remaining_data.raw);
                    next_size_idx += 1;
                    if let Some(stored_block) = provider.and_then(|p| p.query(block_hash)) {
                        // We already have the block, let's skip it.
                        self.reader.source.skip(block_size.into())?;
                        self.remaining_data -= block_size;
                        buffer.resize(stored_block.size.unwrap_usize(), 0);
                        let mut source_file = std::fs::File::open(&stored_block.file)
                            .whatever("unable to open file")?;
                        source_file
                            .seek(std::io::SeekFrom::Start(stored_block.offset.raw))
                            .whatever("unable to seek")?;
                        source_file
                            .read_exact(&mut buffer)
                            .whatever("unable to read block")?;
                    } else {
                        buffer.resize(block_size.try_into().unwrap(), 0);
                        self.reader.source.read_exact(&mut buffer)?;
                        self.remaining_data -= buffer.byte_len();
                        if let Some(format) = block_encoding.compression {
                            buffer = uncompress_bytes(format, &buffer);
                        }
                    }
                } else {
                    // The block has been deduplicated, read from target.
                    assert!(first_idx.raw < idx);
                    let offset = target_offsets[first_idx.raw];
                    let size = target_sizes[first_idx.raw];
                    target.read_block(offset, size, &mut buffer)?;
                }
                // At this point, we have the uncompressed block in the buffer.
                if block_encoding.hash_algorithm.hash(&buffer).raw() != block_hash {
                    bail!("invalid block hash of block {idx} of size {}", buffer.len());
                }
                target_offsets.push(current_target_offset);
                target_sizes.push(buffer.byte_len());
                current_target_offset += buffer.byte_len();
                target.write(&buffer)?;
                payload_hasher.update(&buffer);
            }
        } else {
            loop {
                let read = self.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                target.write(&buffer[..read])?;
                payload_hasher.update(&buffer[..read]);
            }
        }
        if payload_hasher.finalize().raw()
            != self.reader.header.payload_index[self.idx].file_hash.raw
        {
            bail!("payload hash mismatch");
        }
        target.finalize()?;
        skip_until_end(&mut self.reader.source, tags::PAYLOAD)?;
        Ok(())
    }
}

pub trait PayloadTarget: Sized {
    fn write(&mut self, bytes: &[u8]) -> BundleResult<()>;

    #[expect(unused_variables)]
    fn read_block(
        &mut self,
        offset: NumBytes,
        size: NumBytes,
        buffer: &mut Vec<u8>,
    ) -> BundleResult<()> {
        bail!("target does not support reading blocks");
    }

    fn finalize(self) -> BundleResult<()> {
        Ok(())
    }
}

impl PayloadTarget for File {
    fn write(&mut self, bytes: &[u8]) -> BundleResult<()> {
        self.write_all(bytes).whatever("unable to write to target")
    }

    fn read_block(
        &mut self,
        offset: NumBytes,
        size: NumBytes,
        buffer: &mut Vec<u8>,
    ) -> BundleResult<()> {
        let current_position = self
            .stream_position()
            .whatever("unable to get writing position")?;
        self.seek(std::io::SeekFrom::Start(offset.raw))
            .whatever("unable to seek")?;
        buffer.resize(size.unwrap_usize(), 0);
        self.read_exact(buffer).whatever("unable to read")?;
        self.seek(std::io::SeekFrom::Start(current_position))
            .whatever("unable to seek")?;
        Ok(())
    }
}

/// Read next segment or value into vector.
pub fn read_into_vec(
    source: &mut dyn BundleSource,
    output: &mut Vec<u8>,
    head: AtomHead,
    limit: NumBytes,
) -> BundleResult<()> {
    write_atom_head(output, head).unwrap();
    match head {
        AtomHead::Value { length, .. } => {
            if output.byte_len() + length < limit {
                let offset = output.len();
                output.resize(offset + length.raw as usize, 0);
                source
                    .read_exact(&mut output[offset..])
                    .whatever("unable to read value")?;
            } else {
                bail!("value too long");
            }
        }
        AtomHead::Start { tag: start_tag } => loop {
            let inner = expect_atom_head(source)?;
            match inner {
                atom @ AtomHead::End { tag } if tag == start_tag => {
                    write_atom_head(output, atom).unwrap();
                    break;
                }
                atom => {
                    read_into_vec(source, output, atom, limit)?;
                }
            }
        },
        AtomHead::End { tag } => {
            bail!("unbalanced segment end with tag {tag}");
        }
    }
    Ok(())
}

/// Skip atoms until reaching a segment start with the given tag.
#[track_caller]
pub fn skip_until_start(source: &mut dyn BundleSource, tag: Tag) -> BundleResult<AtomHead> {
    loop {
        let head = expect_atom_head(source)?;
        if head.is_start() && head.tag() == tag {
            break Ok(head);
        }
        skip(source, head)?;
    }
}

/// Skip atoms until reaching a segment end with the given tag.
#[track_caller]
pub fn skip_until_end(source: &mut dyn BundleSource, tag: Tag) -> BundleResult<()> {
    loop {
        let head = expect_atom_head(source)?;
        if head.is_end() && head.tag() == tag {
            break Ok(());
        }
        skip(source, head)?;
    }
}

/// Skip atoms until reaching a value with the given tag.
#[track_caller]
pub fn skip_until_value(source: &mut dyn BundleSource, tag: Tag) -> BundleResult<NumBytes> {
    loop {
        let head = expect_atom_head(source)?;
        if let AtomHead::Value {
            tag: value_tag,
            length,
        } = head
        {
            if value_tag == tag {
                break Ok(length);
            }
        }
        skip(source, head)?;
    }
}

/// Expect a segment start.
#[track_caller]
pub fn expect_start(source: &mut dyn BundleSource, tag: Tag) -> BundleResult<AtomHead> {
    match expect_atom_head(source)? {
        atom @ AtomHead::Start { tag: start_tag, .. } if start_tag == tag => Ok(atom),
        atom => bail!("expected start of {tag}, found {atom:?}"),
    }
}

/// Expect the head of an atom.
#[track_caller]
pub fn expect_atom_head(source: &mut dyn BundleSource) -> BundleResult<AtomHead> {
    read_atom_head(source)
        .and_then(|head| head.ok_or_else(|| whatever!("unexpected end of bundle, expected atom")))
}

/// Expect a segment start.
#[track_caller]
pub fn expect_value(source: &mut dyn BundleSource, tag: Tag) -> BundleResult<NumBytes> {
    match expect_atom_head(source)? {
        AtomHead::Value {
            tag: value_tag,
            length,
        } if value_tag == tag => Ok(length),
        atom => bail!("expected value of {tag}, found {atom:?}"),
    }
}

fn uncompress_bytes(format: CompressionFormat, bytes: &[u8]) -> Vec<u8> {
    match format {
        CompressionFormat::Xz => {
            let mut decoder = rugix_compression::XzDecoder::new();
            let mut output = Vec::new();
            decoder.process(bytes, &mut output).unwrap();
            decoder.finalize(&mut output).unwrap();
            output
        }
    }
}
