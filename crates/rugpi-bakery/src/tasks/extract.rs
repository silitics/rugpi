use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use rugpi_common::{loop_dev::LoopDevice, mount::Mounted, Anyhow};
use tempfile::{tempdir, TempDir};
use xscript::{run, Run};

use crate::utils::download;

#[derive(Debug, Parser)]
pub struct ExtractTask {
    /// The image to extract (can be an HTTP URL).
    image: String,
    /// The archive to create with the system files.
    archive: String,
}

impl ExtractTask {
    pub fn run(&self) -> Anyhow<()> {
        let mut image_path = if self.image.starts_with("http") {
            download(&self.image)?
        } else {
            Utf8PathBuf::from(&self.image)
        };
        if image_path.extension() == Some("xz") {
            let decompressed_image_path = image_path.with_extension("");
            if !decompressed_image_path.is_file() {
                eprintln!("Decompressing XZ image...");
                run!(["xz", "-d", "-k", image_path])?;
            }
            image_path = decompressed_image_path;
        }
        eprintln!("Creating `.tar` archive with system files...");
        if let Some(parent) = Utf8Path::new(&self.archive).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let loop_dev = LoopDevice::attach(image_path)?;
        let temp_dir = tempdir()?;
        let temp_dir_path = Utf8Path::from_path(temp_dir.path()).unwrap();
        let _mounted_root = Mounted::mount(loop_dev.partition(2), temp_dir_path)?;
        let _mounted_boot = Mounted::mount(loop_dev.partition(1), temp_dir_path.join("boot"))?;
        run!(["tar", "-c", "-f", &self.archive, "-C", temp_dir_path, "."])?;
        Ok(())
    }
}
