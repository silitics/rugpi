use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use rugpi_common::{
    disk::{repart, stream::ImgStream, PartitionTable},
    maybe_compressed::MaybeCompressed,
    Anyhow,
};

#[derive(Debug, Subcommand)]
pub enum DiskCmd {
    /// Extract partitions from an image.
    ExtractPartitions {
        /// Image to extract partitions from.
        image: PathBuf,
        /// Partitions to extract.
        partitions: Option<Vec<usize>>,
    },
    Repart {
        image: PathBuf,
        schema: PathBuf,
    },
}

/// Read the partition table of a device or image.
#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    cmd: DiskCmd,
}

fn main() -> Anyhow<()> {
    let args = Args::parse();
    match args.cmd {
        DiskCmd::ExtractPartitions { image, partitions } => {
            let reader: Box<dyn Read> = if image == Path::new("-") {
                Box::new(io::stdin())
            } else {
                Box::new(fs::File::open(&image)?)
            };
            let mut stream = ImgStream::new(MaybeCompressed::new(reader)?)?;
            let mut partition_idx = 0;
            while let Some(mut partition) = stream.next_partition()? {
                println!("{partition_idx}  {}", partition.entry());
                if let Some(partitions) = &partitions {
                    if !partitions.contains(&partition_idx) {
                        partition_idx += 1;
                        continue;
                    }
                }
                let mut partition_file = fs::File::create(&format!("p{partition_idx}.part.img"))?;
                io::copy(&mut partition, &mut partition_file)?;
                partition_idx += 1;
            }
        }
        DiskCmd::Repart { image, schema } => {
            let old_table = PartitionTable::read(&image)?;
            let schema = serde_json::from_str(&std::fs::read_to_string(schema)?)?;
            let new_table = repart::repart(&old_table, &schema)?;
            new_table.write(&image)?;
        }
    }

    Ok(())
}
