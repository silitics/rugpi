use std::fs::File;
use std::path::PathBuf;

use clap::Parser;

use reportify::{bail, ResultExt};
use rugix_bundle::format::tags::TagNameResolver;
use rugix_bundle::reader::BundleReader;
use rugix_bundle::source::FileSource;
use rugix_bundle::BundleResult;
use rugix_hashes::HashDigest;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Parser)]
pub enum Cmd {
    /// Create a bundle from a bundle directory.
    Bundle(BundleCmd),
    /// Hash the header of a bundle.
    Hash(HashCmd),
    /// Unpack a payload from a bundle.
    Unpack(UnpackCmd),
    Inspect(InspectCmd),
    /// Print the low-level structure of a bundle.
    #[clap(hide(true))]
    PrintStructure(PrintCmd),
}

#[derive(Debug, Parser)]
pub struct PrintCmd {
    bundle: PathBuf,
}

#[derive(Debug, Parser)]
pub struct BundleCmd {
    /// Source bundle directory.
    src: PathBuf,
    /// Output bundle file.
    dst: PathBuf,
}

#[derive(Debug, Parser)]
pub struct ListCmd {
    bundle: PathBuf,
}

#[derive(Debug, Parser)]
pub struct UnpackCmd {
    #[clap(long)]
    verify_bundle: Option<HashDigest>,
    bundle: PathBuf,
    payload: usize,
    dst: PathBuf,
}

#[derive(Debug, Parser)]
pub struct InspectCmd {
    #[clap(long)]
    verify_bundle: Option<HashDigest>,
    bundle: PathBuf,
}

#[derive(Debug, Parser)]
pub struct HashCmd {
    bundle: PathBuf,
}

fn main() -> BundleResult<()> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Bundle(create_cmd) => {
            rugix_bundle::builder::pack(&create_cmd.src, &create_cmd.dst)?;
        }
        Cmd::Unpack(unpack_cmd) => {
            let source = FileSource::from_unbuffered(File::open(&unpack_cmd.bundle).unwrap());
            let mut reader = BundleReader::start(source, unpack_cmd.verify_bundle)?;
            let mut did_read = false;
            while let Some(payload_reader) = reader.next_payload()? {
                if payload_reader.idx() != unpack_cmd.payload {
                    payload_reader.skip()?;
                } else {
                    println!("unpacking payload...");
                    let target = std::fs::OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .read(true)
                        .write(true)
                        .open(&unpack_cmd.dst)
                        .whatever("unable to open payload target")?;
                    payload_reader.decode_into(target, None)?;
                    did_read = true;
                    break;
                }
            }
            if !did_read {
                bail!("not enough payloads");
            }
        }
        Cmd::PrintStructure(print_cmd) => {
            let mut source = FileSource::from_unbuffered(File::open(&print_cmd.bundle).unwrap());
            rugix_bundle::format::stlv::pretty_print(&mut source, Some(&TagNameResolver)).unwrap();
        }
        Cmd::Hash(hash_cmd) => {
            let hash = rugix_bundle::bundle_hash(&hash_cmd.bundle).unwrap();
            println!("{hash}");
        }
        Cmd::Inspect(inspect_cmd) => {
            let source = FileSource::from_unbuffered(File::open(&inspect_cmd.bundle).unwrap());
            let reader = BundleReader::start(source, inspect_cmd.verify_bundle)?;
            println!("Payloads:");
            for (idx, entry) in reader.header().payload_index.iter().enumerate() {
                if let Some(slot_type) = &entry.type_slot {
                    println!(
                        "  {idx}: slot={:?} file={}",
                        slot_type.slot,
                        HashDigest::new_unchecked(
                            reader.header().hash_algorithm,
                            &entry.file_hash.raw
                        )
                    );
                }
                if let Some(type_execute) = &entry.type_execute {
                    let command = type_execute.handler.join(" ");
                    println!(
                        "  {idx}: execute({command}) file={}",
                        HashDigest::new_unchecked(
                            reader.header().hash_algorithm,
                            &entry.file_hash.raw
                        )
                    );
                }
            }
        }
    }
    Ok(())
}
