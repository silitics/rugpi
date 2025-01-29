use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use clap::Parser;

use byte_calc::{ByteLen, NumBytes};

use rugix_chunker::casync::CasyncChunker;
use rugix_chunker::Chunker;
use rugix_hashes::HashAlgorithm;

/// Command line arguments.
#[derive(Debug, Parser)]
pub struct Args {
    /// Hash algorithm to use for hashing chunks.
    algorithm: HashAlgorithm,
    /// File to chunk.
    file: PathBuf,
}

pub fn main() {
    let args = Args::parse();
    let mut chunk_offset = NumBytes::ZERO;
    let mut chunk_size = NumBytes::ZERO;
    let mut chunk_hasher = args.algorithm.hasher();
    let mut reader = BufReader::new(std::fs::File::open(&args.file).unwrap());
    let mut chunker = CasyncChunker::default();
    loop {
        let buffer = reader.fill_buf().unwrap();
        let offset = chunker
            .scan(buffer)
            .or_else(|| if buffer.is_empty() { Some(0) } else { None });
        chunk_hasher.update(&buffer[..offset.unwrap_or(buffer.len())]);
        let consume = if let Some(offset) = offset {
            chunk_size += NumBytes::from_usize(offset);
            let chunk_digest =
                std::mem::replace(&mut chunk_hasher, args.algorithm.hasher()).finalize();
            println!("Offset: {chunk_offset:#}, Size: {chunk_size:#}, Hash: {chunk_digest}");
            chunk_offset += chunk_size;
            chunk_size = NumBytes::ZERO;
            if buffer.is_empty() {
                break;
            }
            offset
        } else {
            chunk_size += buffer.byte_len();
            buffer.len()
        };
        reader.consume(consume);
    }
}
