use std::io::Read;

use digest::Digest;

pub struct StreamHasher<R, H> {
    reader: R,
    hasher: H,
}

impl<R, H: Digest> StreamHasher<R, H> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            hasher: H::new(),
        }
    }

    pub fn finalize(self) -> digest::Output<H> {
        self.hasher.finalize()
    }
}

impl<R: Read, H: Digest> Read for StreamHasher<R, H> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes_read = self.reader.read(buf)?;
        self.hasher.update(&buf[..bytes_read]);
        Ok(bytes_read)
    }
}
