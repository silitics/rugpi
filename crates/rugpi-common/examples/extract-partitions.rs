use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use clap::Parser;
use rugpi_common::{img_stream::ImgStream, maybe_compressed::MaybeCompressed, Anyhow};

/// Extract partitions from an image stream.
#[derive(Debug, Parser)]
pub struct Args {
    /// The path of the image.
    image: PathBuf,
    /// The partitions to extract.
    partitions: Option<Vec<usize>>,
}

fn main() -> Anyhow<()> {
    let args = Args::parse();
    let reader: Box<dyn Read> = if args.image == Path::new("-") {
        Box::new(io::stdin())
    } else {
        Box::new(fs::File::open(args.image)?)
    };
    let mut stream = ImgStream::new(MaybeCompressed::new(reader)?)?;
    let mut partition_idx = 0;
    while let Some(mut partition) = stream.next_partition()? {
        println!("{partition_idx}  {}", partition.entry());
        if let Some(partitions) = &args.partitions {
            if !partitions.contains(&partition_idx) {
                partition_idx += 1;
                continue;
            }
        }
        let mut partition_file = fs::File::create(&format!("p{partition_idx}.part.img"))?;
        io::copy(&mut partition, &mut partition_file)?;
        partition_idx += 1;
    }

    Ok(())
}
