use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use rugix_bundle::format::tags::TagNameResolver;
use rugix_bundle::source::FileSource;

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
    bundle: PathBuf,
    payload: usize,
    dst: PathBuf,
}

#[derive(Debug, Parser)]
pub struct HashCmd {
    bundle: PathBuf,
}

fn main() {
    let args = Args::parse();
    match args.cmd {
        Cmd::Bundle(create_cmd) => {
            rugix_bundle::builder::pack(&create_cmd.src, &create_cmd.dst).unwrap()
        }
        Cmd::Unpack(_unpack_cmd) => todo!("implement unpacking"),
        Cmd::PrintStructure(print_cmd) => {
            let mut source = FileSource::from_unbuffered(File::open(&print_cmd.bundle).unwrap());
            rugix_bundle::format::stlv::pretty_print(&mut source, Some(&TagNameResolver)).unwrap();
        }
        Cmd::Hash(hash_cmd) => {
            let hash = rugix_bundle::bundle_hash(&hash_cmd.bundle).unwrap();
            println!("{hash}");
        }
    }
}
