use std::{
    collections::HashMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use rugpi_common::{
    boot::grub::{grub_envblk_decode, grub_envblk_encode},
    disk::{blkpg::update_kernel_partitions, repart, stream::ImgStream, PartitionTable},
    maybe_compressed::MaybeCompressed,
    partitions::is_block_dev,
    Anyhow,
};
use xscript::{run, Run};

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
        DiskCmd::Repart { dev, schema, write } => {
            let old_table = PartitionTable::read(&dev)?;
            println!("Disk Size: {}", old_table.disk_size);
            println!("First Usable: {}", old_table.first_usable_block());
            println!("Last Usable: {}", old_table.last_usable_block());
            let schema = serde_json::from_str(&std::fs::read_to_string(schema)?)?;
            if let Some(new_table) = repart::repart(&old_table, &schema)? {
                if write {
                    new_table.write(&dev)?;
                    if is_block_dev(&dev) {
                        update_kernel_partitions(&dev, &old_table, &new_table)?;
                    }
                }
            } else {
                println!("Table has not been changed.");
            }
        }
        DiskCmd::DecodeGrubEnv { env_file } => {
            let data = std::fs::read_to_string(env_file)?;
            let env_blk = grub_envblk_decode(&data);

            println!("{env_blk:?}");
        }
        DiskCmd::EncodeGrubEnv {
            json_file,
            env_file,
        } => {
            let data = serde_json::from_str::<HashMap<String, String>>(&std::fs::read_to_string(
                json_file,
            )?)?;
            std::fs::write(env_file, grub_envblk_encode(&data)?)?;
        }
        DiskCmd::ExtractExt4 { image, dst } => {
            let image = image.canonicalize()?;
            fs::create_dir_all(&dst).ok();
            run!(["/usr/sbin/debugfs", "-R", "rdump / .", image].with_cwd(&dst))?;
        }
        DiskCmd::ExtractFat32 { image, dst } => {
            let image = image.canonicalize()?;
            fs::create_dir_all(&dst).ok();
            run!(["/usr/bin/mcopy", "-i", image, "-snop", "::", dst])?;
        }
    }

    Ok(())
}
