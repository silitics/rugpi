//! Infrastructure for reading partitions from a streamed image.

use std::{
    collections::VecDeque,
    fmt::Display,
    io::{self, Read},
};

use thiserror::Error;

/// Standard sector size is 512 bytes.
const SECTOR_SIZE: usize = 512;
/// Standard sector size of 512 bytes as [`u64`].
const SECTOR_SIZE_U64: u64 = SECTOR_SIZE as u64;

/// Size of the read buffer.
const BUFFER_SIZE: usize = 16 * SECTOR_SIZE;
/// Size of the read buffer as [`u64`].
const BUFFER_SIZE_U64: u64 = BUFFER_SIZE as u64;

/// An image which is being streamed.
pub struct ImgStream<R> {
    /// The inner reader.
    reader: R,
    /// The current position.
    position: u64,
    /// The buffer for reading a sector.
    buffer: Vec<u8>,
    /// The pending partition entries.
    pending: VecDeque<PartitionEntry>,
    /// The extended partition entry of the MBR.
    extended: Option<PartitionEntry>,
}

impl<R: Read> ImgStream<R> {
    pub fn new(reader: R) -> Result<Self, ImgStreamError> {
        let mut this = Self {
            reader,
            position: 0,
            buffer: vec![0; BUFFER_SIZE],
            pending: VecDeque::new(),
            extended: None,
        };
        this.read_next_sector()?;
        if &this.buffer[SECTOR_SIZE - 2..SECTOR_SIZE] != &[0x55, 0xAA] {
            return Err(ImgStreamError::Invalid("invalid magic bytes in MBR"));
        }
        for entry in parse_partition_table(&this.buffer[..SECTOR_SIZE]) {
            if entry.is_extended() {
                if this.extended.is_some() {
                    return Err(ImgStreamError::Invalid(
                        "more than one extended partition entry in MBR",
                    ));
                }
                this.extended = Some(entry.clone());
            }
            this.pending.push_back(entry);
        }
        Ok(this)
    }

    /// Fill the buffer with the next sector.
    fn read_next_sector(&mut self) -> io::Result<()> {
        assert!(
            self.position % SECTOR_SIZE_U64 == 0,
            "invalid sector reading at unaligned position"
        );
        self.reader.read_exact(&mut self.buffer[..SECTOR_SIZE])?;
        self.position += SECTOR_SIZE_U64;
        Ok(())
    }

    /// Advance the reader to the provided entry.
    fn advance_reader(&mut self, entry: &PartitionEntry) -> Result<(), ImgStreamError> {
        let start_position = entry.start_bytes();
        if start_position < self.position {
            return Err(ImgStreamError::Invalid(
                "invalid start sector or unsupported partition order",
            ));
        }
        let skip_bytes = start_position - self.position;
        let skip_unaligned = (skip_bytes % BUFFER_SIZE_U64) as usize;
        if skip_unaligned > 0 {
            // Realign the reader with the sector/buffer size.
            self.reader.read_exact(&mut self.buffer[..skip_unaligned])?;
        }
        for _ in 0..(skip_bytes / BUFFER_SIZE_U64) {
            self.reader.read_exact(&mut self.buffer)?;
        }
        self.position = start_position;
        Ok(())
    }

    /// Return next partition to be read.
    pub fn next_partition(&mut self) -> Result<Option<Partition<'_, R>>, ImgStreamError> {
        loop {
            let Some(entry) = self.pending.pop_front() else {
                return Ok(None);
            };
            // Advance reader to the start of the partition entry.
            self.advance_reader(&entry)?;
            if entry.is_extended() {
                // The entry points to an EBR, read the EBR.
                self.read_next_sector()?;
                let mut entries = parse_partition_table(&self.buffer[..SECTOR_SIZE]);
                if let Some(mut first) = entries.next() {
                    if first.is_extended() {
                        return Err(ImgStreamError::Invalid("invalid first entry of EBR"));
                    }
                    // Address of partition is relative to this EBR.
                    first.start += entry.start;
                    self.pending.push_back(first);
                }
                if let Some(mut second) = entries.next() {
                    if !second.is_extended() {
                        return Err(ImgStreamError::Invalid("invalid second entry of EBR"));
                    }
                    // Address of next EBR is relative to first EBR.
                    second.start += self.extended.as_ref().unwrap().start;
                    self.pending.push_back(second);
                }
            } else {
                break Ok(Some(Partition {
                    stream: self,
                    remaining: entry.size_bytes(),
                    entry,
                }));
            }
        }
    }
}

/// Reader for a partition.
pub struct Partition<'stream, R> {
    /// The underlying image stream.
    stream: &'stream mut ImgStream<R>,
    /// The number of remaining bytes of the partition.
    remaining: u64,
    /// The entry of the partition.
    entry: PartitionEntry,
}

impl<'stream, R> Partition<'stream, R> {
    /// The entry of the partition.
    pub fn entry(&self) -> &PartitionEntry {
        &self.entry
    }

    /// The number of remaining bytes.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
}

impl<'stream, R: Read> Read for Partition<'stream, R> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining < buf.len() as u64 {
            if self.remaining == 0 {
                // Nothing more to read, indicate EOF.
                return Ok(0);
            }
            // Clamp the buffer to the number of bytes remaining in the partition.
            buf = &mut buf[..self.remaining as usize];
        }
        let size = self.stream.reader.read(buf)?;
        self.remaining -= size as u64;
        self.stream.position += size as u64;
        Ok(size)
    }
}

/// Error reading an image stream.
#[derive(Debug, Error)]
pub enum ImgStreamError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Invalid partition table.
    #[error("{0}")]
    Invalid(&'static str),
}

/// Offset of the partition entries.
const PARTITION_ENTRIES_OFFSET: usize = 446;
/// Size of a partition entry.
const PARTITION_ENTRY_SIZE: usize = 16;

/// Parse the entries of the partition table of a boot record.
///
/// # Panics
///
/// Panics in case the given slice does not consist of exactly 512 bytes.
fn parse_partition_table(record: &[u8]) -> impl '_ + Iterator<Item = PartitionEntry> {
    assert_eq!(
        record.len(),
        SECTOR_SIZE,
        "size of boot record must be 512 bytes",
    );
    (0..4)
        .map(|entry_idx| {
            let entry_start = PARTITION_ENTRIES_OFFSET + entry_idx * PARTITION_ENTRY_SIZE;
            let entry_end = entry_start + PARTITION_ENTRY_SIZE;
            let entry_bytes = &record[entry_start..entry_end];
            PartitionEntry::from_bytes(entry_bytes)
        })
        .filter(|entry| !entry.is_free())
}

/// An entry in a partition table using LBA addressing.
#[derive(Debug, Clone)]
pub struct PartitionEntry {
    /// The kind of the partition.
    kind: u8,
    /// The start sector.
    start: u32,
    /// The size of the partition in sectors.
    size: u32,
}

impl PartitionEntry {
    /// Parse a partition entry from the given bytes.
    ///
    /// # Panics
    ///
    /// Panics in case the given slice does not consist of exactly 16 bytes.
    fn from_bytes(entry: &[u8]) -> Self {
        assert_eq!(
            entry.len(),
            PARTITION_ENTRY_SIZE,
            "size of partition entry must be 16 bytes"
        );
        let kind = entry[4];
        let start = u32::from_le_bytes(entry[8..12].try_into().unwrap());
        let size = u32::from_le_bytes(entry[12..16].try_into().unwrap());
        Self { kind, start, size }
    }

    /// Indicates whether the partition entry is free.
    fn is_free(&self) -> bool {
        // Free entries in the partition table have their type set to 0x00.
        self.kind == 0x00
    }

    /// Indicates whether the partition entry points to an EBR.
    fn is_extended(&self) -> bool {
        // Technically, 0x05 would use CHS addressing, but modern tools provide
        // LBA addressing anyway and converting is non-trivial, hence, we will
        // just rely on LBA addressing everywhere.
        self.kind == 0x05 || self.kind == 0x0F
    }

    /// The type of the partition.
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// The size of the partition in bytes.
    pub fn size_bytes(&self) -> u64 {
        (self.size as u64) * (SECTOR_SIZE as u64)
    }

    /// The start of the partition in bytes.
    pub fn start_bytes(&self) -> u64 {
        (self.start as u64) * (SECTOR_SIZE as u64)
    }
}

impl Display for PartitionEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "kind: {:#04x}, start: {}, size: {}",
            self.kind, self.start, self.size
        ))
    }
}
