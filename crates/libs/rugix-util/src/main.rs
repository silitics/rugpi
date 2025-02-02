use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use reportify::{Report, ResultExt};
use rugix_common::boot::grub::{grub_envblk_decode, grub_envblk_encode};
use rugix_common::disk::blkdev::is_block_device;
use rugix_common::disk::blkpg::update_kernel_partitions;
use rugix_common::disk::stream::ImgStream;
use rugix_common::disk::{repart, PartitionTable};
use rugix_common::maybe_compressed::MaybeCompressed;
use xscript::{run, Run};

reportify::new_whatever_type! {
    RugixUtilError
}

#[derive(Debug, Subcommand)]
pub enum DiskCmd {
    /// Extract partitions from an image.
    ExtractPartitions {
        /// Image to extract partitions from.
        image: PathBuf,
        /// Partitions to extract.
        partitions: Option<Vec<usize>>,
    },
    ExtractExt4 {
        image: PathBuf,
        dst: PathBuf,
    },
    ExtractFat32 {
        image: PathBuf,
        dst: PathBuf,
    },
    Repart {
        #[clap(long)]
        write: bool,
        dev: PathBuf,
        schema: PathBuf,
    },
    DecodeGrubEnv {
        env_file: PathBuf,
    },
    EncodeGrubEnv {
        json_file: PathBuf,
        env_file: PathBuf,
    },
}

/// Read the partition table of a device or image.
#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    cmd: DiskCmd,
}

fn main() -> Result<(), Report<RugixUtilError>> {
    let args = Args::parse();
    match args.cmd {
        DiskCmd::ExtractPartitions { image, partitions } => {
            let reader: Box<dyn Read> = if image == Path::new("-") {
                Box::new(io::stdin())
            } else {
                Box::new(fs::File::open(&image).whatever("unable to open image file")?)
            };
            let mut stream =
                ImgStream::new(MaybeCompressed::new(reader).whatever("unable to open image")?)
                    .whatever("unable to open image")?;
            let mut partition_idx = 0;
            while let Some(mut partition) = stream
                .next_partition()
                .whatever("unable to get next partition")?
            {
                println!("{partition_idx}  {}", partition.entry());
                if let Some(partitions) = &partitions {
                    if !partitions.contains(&partition_idx) {
                        partition_idx += 1;
                        continue;
                    }
                }
                let mut partition_file = fs::File::create(&format!("p{partition_idx}.part.img"))
                    .whatever("unable to create partition file")?;
                io::copy(&mut partition, &mut partition_file)
                    .whatever("unable to write partition file")?;
                partition_idx += 1;
            }
        }
        DiskCmd::Repart { dev, schema, write } => {
            let old_table =
                PartitionTable::read(&dev).whatever("unable to read partition table")?;
            println!("Disk Size: {}", old_table.disk_size);
            println!("First Usable: {}", old_table.first_usable_block());
            println!("Last Usable: {}", old_table.last_usable_block());
            let schema = serde_json::from_str(
                &std::fs::read_to_string(schema).whatever("unable to read partition schema")?,
            )
            .whatever("unable to parse partition schema")?;
            if let Some(new_table) = repart::repart(&old_table, &schema)
                .whatever("unable to compute new partition table")?
            {
                if write {
                    new_table
                        .write(&dev)
                        .whatever("unable to write new partition table")?;
                    if is_block_device(&dev)
                        .whatever("unable to determine whether device is a block device")?
                    {
                        update_kernel_partitions(&dev, &old_table, &new_table)
                            .whatever("unable to update partitions in the Kernel")?;
                    }
                }
            } else {
                println!("Table has not been changed.");
            }
        }
        DiskCmd::DecodeGrubEnv { env_file } => {
            let data = std::fs::read_to_string(env_file)
                .whatever("unable to read Grub environment file")?;
            let env_blk =
                grub_envblk_decode(&data).whatever("unable to decode Grub environment file")?;

            println!("{env_blk:?}");
        }
        DiskCmd::EncodeGrubEnv {
            json_file,
            env_file,
        } => {
            let data = serde_json::from_str::<HashMap<String, String>>(
                &std::fs::read_to_string(json_file)
                    .whatever("unable to read JSON environment variables")?,
            )
            .whatever("unable to parse JSON")?;
            std::fs::write(
                env_file,
                grub_envblk_encode(&data).whatever("unable to encode Grub environment")?,
            )
            .whatever("unable to write Grub environment file")?;
        }
        DiskCmd::ExtractExt4 { image, dst } => {
            let image = image
                .canonicalize()
                .whatever("unable to canonicalize image file")?;
            fs::create_dir_all(&dst).ok();
            run!(["/usr/sbin/debugfs", "-R", "rdump / .", image].with_cwd(&dst))
                .whatever("unable to extract files")?;
        }
        DiskCmd::ExtractFat32 { image, dst } => {
            let image = image
                .canonicalize()
                .whatever("unable to canonicalize image file")?;
            fs::create_dir_all(&dst).ok();
            run!(["/usr/bin/mcopy", "-i", image, "-snop", "::", dst])
                .whatever("unable to extract files")?;
        }
    }

    Ok(())
}
