//! Functionality for baking layers and images.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail};
use rugpi_common::{loop_dev::LoopDevice, mount::Mounted, Anyhow};
use tempfile::tempdir;
use url::Url;
use xscript::{run, Run};

use crate::{
    project::{config::Architecture, library::LayerIdx, Project},
    utils::{
        caching::{download, Hasher},
        prelude::*,
    },
};

pub mod customize;
pub mod image;
pub mod targets;

pub fn bake_image(project: &Project, image: &str, output: &Path) -> Anyhow<()> {
    let image_config = project
        .config
        .images
        .get(image)
        .ok_or_else(|| anyhow!("unable to find image {image}"))?;
    info!("baking image `{image}`");
    let layer_bakery = LayerBakery::new(project, image_config.architecture);
    let baked_layer = layer_bakery.bake_root(&image_config.layer)?;
    image::make_image(image_config, &baked_layer, output)
}

pub struct LayerBakery<'p> {
    project: &'p Project,
    arch: Architecture,
}

impl<'p> LayerBakery<'p> {
    pub fn new(project: &'p Project, arch: Architecture) -> Self {
        Self { project, arch }
    }

    pub fn bake_root(&self, layer: &str) -> Anyhow<PathBuf> {
        let library = self.project.library()?;
        let Some(layer) = library.lookup_layer(library.repositories.root_repository, layer) else {
            bail!("unable to find layer {layer}");
        };
        self.bake(layer)
    }

    pub fn bake(&self, layer: LayerIdx) -> Anyhow<PathBuf> {
        let repositories = &self.project.repositories()?.repositories;
        let library = self.project.library()?;
        let layer = &library.layers[layer];
        info!("baking layer `{}`", layer.name);
        let Some(config) = layer.config(self.arch) else {
            bail!("no layer configuration for architecture `{}`", self.arch);
        };
        let mut layer_id = Hasher::new();
        layer_id.push("layer", &layer.name);
        layer_id.push("repository", repositories[layer.repo].source.id.as_str());
        layer_id.push("arch", self.arch.as_str());
        if let Some(url) = &config.url {
            layer_id.push("url", url);
            let layer_id = layer_id.finalize();
            let system_tar = self
                .project
                .dir
                .join(format!(".rugpi/layers/{layer_id}/system.tar"));
            if !system_tar.exists() {
                extract(self.project, url, &system_tar)?;
            }
            Ok(system_tar)
        } else if let Some(parent) = &config.parent {
            layer_id.push("parent", parent);
            let Some(parent) = library.lookup_layer(layer.repo, parent) else {
                bail!("unable to find layer `{parent}`");
            };
            let src = self.bake(parent)?;
            let layer_id = layer_id.finalize();
            let layer_path = PathBuf::from(format!(".rugpi/layers/{layer_id}"));
            let target = self.project.dir.join(&layer_path).join("system.tar");
            fs::create_dir_all(target.parent().unwrap()).ok();
            customize::customize(
                self.project,
                self.arch,
                layer,
                Some(&src),
                &target,
                &layer_path,
            )?;
            Ok(target)
        } else if config.root {
            layer_id.push("bare", "true");
            let layer_id = layer_id.finalize();
            let layer_path = PathBuf::from(format!(".rugpi/layers/{layer_id}"));
            let target = self.project.dir.join(&layer_path).join("system.tar");
            fs::create_dir_all(target.parent().unwrap()).ok();
            customize::customize(self.project, self.arch, layer, None, &target, &layer_path)?;
            Ok(target)
        } else {
            bail!("invalid layer configuration")
        }
    }
}

fn extract(project: &Project, image_url: &str, layer_path: &Path) -> Anyhow<()> {
    let image_url = image_url.parse::<Url>()?;
    let mut image_path = match image_url.scheme() {
        "file" => {
            let mut image_path = project.dir.to_path_buf();
            image_path.push(image_url.path().strip_prefix('/').unwrap());
            image_path
        }
        _ => download(&image_url)?,
    };
    if image_path.extension() == Some("xz".as_ref()) {
        let decompressed_image_path = image_path.with_extension("");
        if !decompressed_image_path.is_file() {
            info!("decompressing XZ image");
            run!(["xz", "-d", "-k", image_path])?;
        }
        image_path = decompressed_image_path;
    }
    if image_path.extension() == Some("gz".as_ref()) {
        let decompressed_image_path = image_path.with_extension("");
        if !decompressed_image_path.is_file() {
            info!("decompressing GZ image");
            run!(["gzip", "-d", "-k", image_path])?;
        }
        image_path = decompressed_image_path;
    }
    if let Some(parent) = layer_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    if image_path.extension() == Some("tar".as_ref()) {
        info!("Copying root filesystem {image_path:?}");
        fs::copy(image_path, layer_path)?;
    } else {
        info!("creating `.tar` archive with system files");
        let loop_dev = LoopDevice::attach(image_path)?;
        let temp_dir = tempdir()?;
        let temp_dir_path = temp_dir.path();
        let system_dir = temp_dir_path.join("roots/system");
        let boot_dir = temp_dir_path.join("roots/boot");
        std::fs::create_dir_all(&system_dir)?;
        std::fs::create_dir_all(&boot_dir)?;
        let _mounted_root = Mounted::mount(loop_dev.partition(2), &system_dir)?;
        let _mounted_boot =
            Mounted::mount(loop_dev.partition(1), temp_dir_path.join("roots/boot"))?;
        run!(["tar", "-c", "-f", &layer_path, "-C", temp_dir_path, "."])?;
    }
    Ok(())
}
