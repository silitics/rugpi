//! A stream which may be compressed.

use std::io::{self, Read};

use xz2::read::XzDecoder;

const XZ_MAGIC: &[u8] = &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00];

pub struct MaybeCompressed<R: Read> {
    inner: MaybeCompressedInner<R>,
}

impl<R: Read> MaybeCompressed<R> {
    pub fn new(reader: R) -> io::Result<Self> {
        let mut reader = PeekReader::new(reader);
        let magic = reader.peek(6)?;
        if magic == XZ_MAGIC {
            Ok(Self {
                inner: MaybeCompressedInner::Xz(XzDecoder::new(reader)),
            })
        } else {
            Ok(Self {
                inner: MaybeCompressedInner::Uncompressed(reader),
            })
        }
    }
}

impl<R: Read> Read for MaybeCompressed<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            MaybeCompressedInner::Uncompressed(reader) => reader.read(buf),
            MaybeCompressedInner::Xz(reader) => reader.read(buf),
        }
    }
}

enum MaybeCompressedInner<R: Read> {
    Uncompressed(PeekReader<R>),
    Xz(XzDecoder<PeekReader<R>>),
}

/// The default size of the buffer of [`PeekReader`].
const DEFAULT_PEEK_BUFFER_SIZE: usize = 8192;

/// A reader from which bytes can be peeked.
struct PeekReader<R> {
    /// The underlying reader.
    reader: R,
    /// The peek buffer.
    buffer: Vec<u8>,
    /// The number of bytes in the peek buffer.
    filled: usize,
    /// The number of bytes peeked from the peek buffer.
    peeked: usize,
    /// The number of bytes consumed from the peek buffer.
    consumed: usize,
}

impl<R: Read> PeekReader<R> {
    /// Create a new [`PeekReader`] with the default buffer size.
    ///
    /// Guarantees that at least 8192 bytes can be peeked.
    pub fn new(reader: R) -> Self {
        Self::with_capacity(DEFAULT_PEEK_BUFFER_SIZE, reader)
    }

    /// Create a new [`PeekReader`] with the given buffer size.
    pub fn with_capacity(capacity: usize, reader: R) -> Self {
        Self {
            reader,
            buffer: vec![0; capacity],
            filled: 0,
            peeked: 0,
            consumed: 0,
        }
    }

    /// The free space in the peek buffer.
    fn free(&self) -> usize {
        self.buffer.len() - self.filled
    }

    /// Fill the buffer with at least `size` peekable bytes.
    ///
    /// If the underlying reader reaches EOF, the buffer may contain less bytes.
    fn fill_buffer(&mut self, size: usize) -> io::Result<()> {
        if self.free() < size {
            panic!("cannot peek more than {} bytes", self.buffer.len());
        }
        while self.filled - self.peeked < size {
            let read = self.reader.read(&mut self.buffer[self.filled..])?;
            if read == 0 {
                break;
            }
            self.filled += read;
        }
        Ok(())
    }

    /// Reset the peek buffer.
    fn reset_buffer(&mut self) {
        self.filled = 0;
        self.consumed = 0;
        self.peeked = 0;
    }

    /// Peek the given number of bytes.
    pub fn peek(&mut self, size: usize) -> io::Result<&[u8]> {
        self.fill_buffer(size)?;
        let chunk = &self.buffer[self.peeked..(self.peeked + size).min(self.buffer.len())];
        self.peeked += chunk.len();
        Ok(chunk)
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.consumed < self.filled {
            let size = (self.filled - self.consumed).min(buf.len());
            buf[..size].copy_from_slice(&self.buffer[self.consumed..self.consumed + size]);
            self.consumed += size;
            if self.consumed == self.filled {
                self.reset_buffer();
            }
            Ok(size)
        } else {
            self.reader.read(buf)
        }
    }
}
