use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use reportify::{whatever, Report, ResultExt};
use rugpi_common::artifact::format::encode::{self, Encode};
use rugpi_common::artifact::format::stlv::{
    self, write_atom_head, write_close_segment, write_open_segment, AtomHead, SkipSeek,
};
use rugpi_common::artifact::format::tags::{self, TagNameResolver};
use rugpi_common::artifact::format::{ArtifactHeader, FragmentHeader, FragmentInfo, Hash};
use sha2::Digest;

reportify::new_whatever_type! {
    CliError
}

#[derive(Debug, Clone, Parser)]
pub struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Clone, Parser)]
pub enum Cmd {
    /// Pack an artifact.
    Pack(PackCmd),
    /// Print the structure of the artifact.
    Print(PrintCmd),
}

#[derive(Debug, Clone, Parser)]
pub struct PrintCmd {
    artifact: PathBuf,
}

#[derive(Debug, Clone, Parser)]
pub struct PackCmd {
    /// Path to the artifact.
    artifact: PathBuf,
    /// Directory containing the unpacked artifact.
    directory: PathBuf,
}

pub fn main() -> Result<(), Report<CliError>> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Pack(cmd) => {
            let mut fragment_ids = Vec::new();
            for entry in std::fs::read_dir(cmd.directory.join("fragments"))
                .whatever("unable to read fragments from directory")?
            {
                let entry = entry.whatever("unable to obtain fragment directory entry")?;
                let file_name = entry.file_name();
                let id = file_name
                    .to_str()
                    .ok_or_else(|| whatever!("invalid UTF-8 in fragment id"))?
                    .to_owned();
                if id.chars().any(|c| !c.is_ascii_digit()) {
                    eprintln!("ignoring fragments/{id}");
                    continue;
                }
                fragment_ids.push(id);
            }
            fragment_ids.sort();
            let mut fragment_infos = Vec::new();
            let mut fragment_headers = Vec::new();
            let mut offset = 0;
            for fragment in &fragment_ids {
                let fragment_path = cmd.directory.join("fragments").join(fragment);
                let payload_path = fragment_path.join("payload");
                let payload_size = payload_path
                    .metadata()
                    .whatever("unable to read metadata of fragment payload")?
                    .len();
                let header = FragmentHeader {};
                let mut hasher = sha2::Sha512_256::new();
                let mut reader = BufReader::new(
                    File::open(&payload_path).whatever("unable to open payload file")?,
                );
                loop {
                    let buffer = reader.fill_buf().whatever("unable to fill buffer")?;
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
                    // metadata: Metadata::default(),
                    filename: read_optional_string(&fragment_path.join("filename"))
                        .whatever("unable to read fragment filename")?,
                    offset: Some(offset),
                    slot: read_optional_string(&fragment_path.join("slot"))
                        .whatever("unable to read fragment slot")?,
                    header_hash,
                    payload_hash,
                });
                offset += AtomHead::open(tags::FRAGMENT).atom_size();
                offset += encoded_header.len() as u64;
                offset += AtomHead::value(tags::FRAGMENT_PAYLOAD, payload_size).atom_size();
                offset += AtomHead::close(tags::FRAGMENT).atom_size();
                fragment_headers.push(encoded_header);
            }
            let header = ArtifactHeader {
                fragments: fragment_infos,
            };
            let mut writer = BufWriter::new(
                File::create(&cmd.artifact).whatever("unable to create artifact file")?,
            );

            fn report_write_result<T>(result: Result<T, io::Error>) -> Result<T, Report<CliError>> {
                result.whatever("unable to write to artifact file")
            }

            report_write_result(write_open_segment(&mut writer, tags::ARTIFACT))?;
            report_write_result(header.encode(&mut writer, tags::ARTIFACT_HEADER))?;
            report_write_result(write_open_segment(&mut writer, tags::FRAGMENTS))?;
            for (idx, fragment) in fragment_ids.iter().enumerate() {
                report_write_result(write_open_segment(&mut writer, tags::FRAGMENT))?;
                report_write_result(writer.write_all(&fragment_headers[idx]))?;
                let payload = cmd
                    .directory
                    .join("fragments")
                    .join(fragment)
                    .join("payload");
                let size = payload
                    .metadata()
                    .whatever("unable to get payload metadata")?
                    .len();
                report_write_result(write_atom_head(
                    &mut writer,
                    AtomHead::value(tags::FRAGMENT_PAYLOAD, size),
                ))?;
                io::copy(
                    &mut File::open(&payload).whatever("unable to open fragment payload")?,
                    &mut writer,
                )
                .whatever("unable to copy fragment payload")?;
                report_write_result(write_close_segment(&mut writer, tags::FRAGMENT))?;
            }
            report_write_result(write_close_segment(&mut writer, tags::FRAGMENTS))?;
            report_write_result(write_close_segment(&mut writer, tags::ARTIFACT))?;
        }
        Cmd::Print(cmd) => {
            let file = fs::File::open(&cmd.artifact).whatever("unable to open artifact file")?;
            let mut reader = BufReader::new(file);
            stlv::pretty_print::<_, SkipSeek>(&mut reader, Some(&TagNameResolver))
                .whatever("unable to pretty print artifact file")?;
        }
    }
    Ok(())
}

fn read_optional_string(path: &Path) -> io::Result<Option<String>> {
    if path.exists() {
        Ok(Some(fs::read_to_string(path)?.trim().to_owned()))
    } else {
        Ok(None)
    }
}
