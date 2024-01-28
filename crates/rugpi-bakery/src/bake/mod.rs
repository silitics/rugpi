//! Functionality for baking layers and images.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail};
use rugpi_common::{loop_dev::LoopDevice, mount::Mounted, Anyhow};
use tempfile::tempdir;
use xscript::{run, Run};

use crate::{
    project::{config::Architecture, Project},
    utils::caching::{download, sha1},
};

pub mod customize;
pub mod image;

pub fn bake_image(project: &Project, image: &str, output: &Path) -> Anyhow<()> {
    let image_config = project
        .config
        .images
        .get(image)
        .ok_or_else(|| anyhow!("unable to find image {image}"))?;
    let baked_layer = bake_layer(project, image_config.architecture, &image_config.layer)?;
    image::make_image(image_config, &baked_layer, output)
}

pub fn bake_layer(project: &Project, arch: Architecture, layer_name: &str) -> Anyhow<PathBuf> {
    let library = project.library()?;
    let layer = &library.layers[library
        .lookup_layer(library.repositories.root_repository, layer_name)
        .unwrap()];
    let layer_config = layer.config(arch).unwrap();
    if let Some(url) = &layer_config.url {
        let layer_id = sha1(url);
        let system_tar = project
            .dir
            .join(format!(".rugpi/layers/{layer_id}/system.tar"));
        if !system_tar.exists() {
            extract(url, &system_tar)?;
        }
        Ok(system_tar)
    } else if let Some(parent) = &layer_config.parent {
        let src = bake_layer(project, arch, parent)?;
        let mut layer_string = layer_name.to_owned();
        layer_string.push('.');
        layer_string.push_str(arch.as_str());
        layer_string.push('.');
        layer_string.push_str(library.repositories[layer.repo].source.id.as_str());
        let layer_id = sha1(&layer_string);
        let layer_path = PathBuf::from(format!(".rugpi/layers/{layer_id}"));
        let target = project.dir.join(&layer_path).join("system.tar");
        fs::create_dir_all(target.parent().unwrap()).ok();
        customize::customize(project, arch, layer, &src, &target, &layer_path)?;
        Ok(target)
    } else {
        bail!("invalid layer configuration")
    }
}

fn extract(image_url: &str, layer_path: &Path) -> Anyhow<()> {
    let mut image_path = download(image_url)?;
    if image_path.extension() == Some("xz".as_ref()) {
        let decompressed_image_path = image_path.with_extension("");
        if !decompressed_image_path.is_file() {
            eprintln!("Decompressing XZ image...");
            run!(["xz", "-d", "-k", image_path])?;
        }
        image_path = decompressed_image_path;
    }
    eprintln!("Creating `.tar` archive with system files...");
    if let Some(parent) = layer_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let loop_dev = LoopDevice::attach(image_path)?;
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path();
    let _mounted_root = Mounted::mount(loop_dev.partition(2), temp_dir_path)?;
    let _mounted_boot = Mounted::mount(loop_dev.partition(1), temp_dir_path.join("boot"))?;
    run!(["tar", "-c", "-f", &layer_path, "-C", temp_dir_path, "."])?;
    Ok(())
}
