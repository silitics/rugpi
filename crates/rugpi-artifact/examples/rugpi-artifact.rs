use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
};

use clap::Parser;
use rugpi_artifact::format::{
    encode::{self, Encode},
    stlv::{write_atom_head, write_close_segment, write_open_segment, AtomHead},
    tags, ArtifactHeader, FragmentHeader, FragmentInfo, Hash, Metadata,
};
use sha2::Digest;

#[derive(Debug, Clone, Parser)]
pub struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Clone, Parser)]
pub enum Cmd {
    /// Create an artifact.
    Create(CreateCmd),
}

#[derive(Debug, Clone, Parser)]
pub struct CreateCmd {
    /// File containing artifact metadata.
    #[clap(long)]
    metadata: Option<PathBuf>,
    /// Path to the artifact.
    artifact: PathBuf,
    /// Paths to the fragments of the artifact.
    fragments: Vec<PathBuf>,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Create(cmd) => {
            let metadata = cmd
                .metadata
                .map(|path| {
                    Ok::<_, Box<dyn Error>>(Metadata {
                        map: serde_json::from_str(&std::fs::read_to_string(path)?)?,
                    })
                })
                .transpose()?
                .unwrap_or_default();
            let mut fragment_infos = Vec::new();
            let mut fragment_headers = Vec::new();
            let mut offset = 0;
            for fragment in &cmd.fragments {
                let size = fragment.metadata()?.len();
                let header = FragmentHeader {
                    compression: None,
                    encoded_index: None,
                    decoded_index: None,
                };
                let mut hasher = sha2::Sha512_256::new();
                let mut reader = BufReader::new(File::open(fragment)?);
                loop {
                    let buffer = reader.fill_buf()?;
                    if buffer.is_empty() {
                        break;
                    }
                    hasher.update(buffer);
                    let consumed = buffer.len();
                    reader.consume(consumed);
                }
                let payload_digest = hasher.finalize();
                let payload_hash = Hash {
                    algorithm: "sha512_256".to_owned(),
                    digest: payload_digest.as_slice().to_vec().into(),
                };
                let encoded_header = encode::to_vec(header, tags::FRAGMENT_HEADER);
                let header_digest = sha2::Sha512_256::digest(&encoded_header);
                let header_hash = Hash {
                    algorithm: "sha512_256".to_owned(),
                    digest: header_digest.as_slice().to_vec().into(),
                };
                fragment_infos.push(FragmentInfo {
                    metadata: Metadata::default(),
                    filename: fragment.to_str().unwrap().to_owned(),
                    offset: Some(offset),
                    slot: None,
                    header_hash,
                    payload_hash,
                });
                offset += AtomHead::open(tags::FRAGMENT).atom_size();
                offset += encoded_header.len() as u64;
                offset += AtomHead::value(tags::FRAGMENT_PAYLOAD, size).atom_size();
                offset += AtomHead::close(tags::FRAGMENT).atom_size();
                fragment_headers.push(encoded_header);
            }
            let header = ArtifactHeader {
                metadata,
                fragments: fragment_infos,
            };
            let mut writer = BufWriter::new(File::create(&cmd.artifact)?);
            write_open_segment(&mut writer, tags::ARTIFACT)?;
            header.encode(&mut writer, tags::ARTIFACT_HEADER)?;
            write_open_segment(&mut writer, tags::FRAGMENTS)?;
            for (idx, fragment) in cmd.fragments.iter().enumerate() {
                write_open_segment(&mut writer, tags::FRAGMENT)?;
                writer.write_all(&fragment_headers[idx])?;
                let size = fragment.metadata()?.len();
                write_atom_head(&mut writer, AtomHead::value(tags::FRAGMENT_PAYLOAD, size))?;
                io::copy(&mut File::open(fragment)?, &mut writer)?;
                write_close_segment(&mut writer, tags::FRAGMENT)?;
            }
            write_close_segment(&mut writer, tags::FRAGMENTS)?;
            write_close_segment(&mut writer, tags::ARTIFACT)?;
        }
    }
    Ok(())
}
