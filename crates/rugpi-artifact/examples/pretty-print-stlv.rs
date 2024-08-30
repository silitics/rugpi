use std::{error::Error, fs, io::BufReader, path::PathBuf};

use clap::Parser;
use rugpi_artifact::format::{
    stlv::{self, SkipSeek},
    tags::TagNameResolver,
};

/// Pretty print an STLV stream.
#[derive(Debug, Clone, Parser)]
pub struct Args {
    /// File with the STLV stream to pretty print.
    file: PathBuf,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let file = fs::File::open(&args.file)?;
    let mut reader = BufReader::new(file);
    stlv::pretty_print::<_, SkipSeek>(&mut reader, Some(&TagNameResolver))?;
    Ok(())
}
