//! Functionality for baking layers and images.

use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use layer::FrozenLayer;
use reportify::{bail, whatever, ResultExt};
use rugix_bundle::manifest::{self, BundleManifest, ChunkerAlgorithm};
use rugix_common::loop_dev::LoopDevice;
use rugix_common::mount::Mounted;
use tempfile::tempdir;
use tracing::info;
use url::Url;
use xscript::{run, Run};

use crate::config::systems::{Architecture, Target};
use crate::project::library::LayerIdx;
use crate::project::ProjectRef;
use crate::utils::caching::{download, Hasher};
use crate::BakeryResult;

pub mod customize;
pub mod layer;
pub mod system;
pub mod targets;

pub fn bake_system(project: &ProjectRef, system: &str, output: &Path) -> BakeryResult<()> {
    let system_config = project
        .config()
        .get_system_config(system)
        .ok_or_else(|| whatever!("unable to find image {system}"))?;
    info!("baking image `{system}`");
    let layer_bakery = LayerBakery::new(project, system_config.architecture);
    let baked_layer = layer_bakery.bake_root(&system_config.layer)?;
    let frozen = FrozenLayer::new(system_config.layer.clone(), baked_layer);
    system::make_system(system_config, &frozen, output)
}

pub struct LayerBakery<'p> {
    project: &'p ProjectRef,
    arch: Architecture,
}

impl<'p> LayerBakery<'p> {
    pub fn new(project: &'p ProjectRef, arch: Architecture) -> Self {
        Self { project, arch }
    }

    pub fn bake_root(&self, layer: &str) -> BakeryResult<PathBuf> {
        let library = self.project.library()?;
        let Some(layer) = library.lookup_layer(library.repositories.root_repository, layer) else {
            bail!("unable to find layer {layer}");
        };
        self.bake(layer)
    }

    pub fn bake(&self, layer: LayerIdx) -> BakeryResult<PathBuf> {
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
                .dir()
                .join(format!(".rugix/layers/{layer_id}/system.tar"));
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
            let layer_path = PathBuf::from(format!(".rugix/layers/{layer_id}"));
            let target = self.project.dir().join(&layer_path).join("system.tar");
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
        } else if config.root.unwrap_or(false) {
            layer_id.push("bare", "true");
            let layer_id = layer_id.finalize();
            let layer_path = PathBuf::from(format!(".rugix/layers/{layer_id}"));
            let target = self.project.dir().join(&layer_path).join("system.tar");
            fs::create_dir_all(target.parent().unwrap()).ok();
            customize::customize(self.project, self.arch, layer, None, &target, &layer_path)?;
            Ok(target)
        } else {
            bail!("invalid layer configuration")
        }
    }
}

fn extract(project: &ProjectRef, image_url: &str, layer_path: &Path) -> BakeryResult<()> {
    let image_url = image_url
        .parse::<Url>()
        .whatever("unable to parse image URL")?;
    let mut image_path = match image_url.scheme() {
        "file" => {
            let mut image_path = project.dir().to_path_buf();
            image_path.push(image_url.path().strip_prefix('/').unwrap());
            image_path
        }
        _ => download(&image_url)?,
    };
    if image_path.extension() == Some("xz".as_ref()) {
        let decompressed_image_path = image_path.with_extension("");
        if !decompressed_image_path.is_file() {
            info!("decompressing XZ image");
            run!(["xz", "-d", "-k", image_path]).whatever("unable to decompress image")?;
        }
        image_path = decompressed_image_path;
    }
    if image_path.extension() == Some("gz".as_ref()) {
        let decompressed_image_path = image_path.with_extension("");
        if !decompressed_image_path.is_file() {
            info!("decompressing GZ image");
            run!(["gzip", "-d", "-k", image_path]).whatever("unable to decompress image")?;
        }
        image_path = decompressed_image_path;
    }
    if let Some(parent) = layer_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).whatever("unable to create layer path")?;
        }
    }
    let temp_dir = tempdir().whatever("unable to create temporary directory")?;
    let temp_dir_path = temp_dir.path();
    let system_dir = temp_dir_path.join("roots/system");
    let boot_dir = temp_dir_path.join("roots/boot");
    std::fs::create_dir_all(&system_dir).whatever("unable to create system directory")?;
    std::fs::create_dir_all(&boot_dir).whatever("unable to create boot directory")?;
    if image_path.extension() == Some("tar".as_ref()) {
        info!("Copying root filesystem {image_path:?}");
        run!(["tar", "-x", "-f", &image_path, "-C", system_dir])
            .whatever("unable to extract root file system")?;
        run!(["tar", "-c", "-f", &layer_path, "-C", temp_dir_path, "."])
            .whatever("unable to create layer tar file")?;
    } else {
        info!("creating `.tar` archive with system files");
        let loop_dev = LoopDevice::attach(image_path).whatever("unable to setup loop device")?;
        let _mounted_root = Mounted::mount(loop_dev.partition(2), &system_dir)
            .whatever("unable to mount system partition")?;
        let _mounted_boot = Mounted::mount(loop_dev.partition(1), temp_dir_path.join("roots/boot"))
            .whatever("unable to mount boot partition")?;
    }
    run!(["tar", "-c", "-f", &layer_path, "-C", temp_dir_path, "."])
        .whatever("unable to create layer tar file")?;
    Ok(())
}

/// Bundle options.
#[derive(Args, Clone, Debug)]
pub struct BundleOpts {
    /// Disable compression of the bundle.
    #[clap(long)]
    without_compression: bool,
}

pub fn bake_bundle(
    project: &ProjectRef,
    system: &str,
    system_path: &Path,
    output: &Path,
    opts: &BundleOpts,
) -> BakeryResult<()> {
    let bundle_dir = tempdir().whatever("unable to create temporary directory")?;
    let bundle_dir = bundle_dir.path();
    let system_config = project.config().resolve_system_config(system)?;
    let config = match system_config.target.clone().unwrap_or(Target::Unknown) {
        Target::GenericGrubEfi => efi_bundle_config(opts),
        Target::RpiTryboot => rpi_bundle_config(opts),
        Target::RpiUboot => rpi_bundle_config(opts),
        Target::Unknown => bail!("cannot bake bundles for unknown targets"),
    };
    std::fs::write(
        bundle_dir.join("rugix-bundle.toml"),
        toml::to_string(&config).unwrap(),
    )
    .whatever("unable to write bundle config")?;
    std::os::unix::fs::symlink(
        system_path
            .join("filesystems")
            .canonicalize()
            .whatever("unable to canonicalize filesystems directory")?,
        bundle_dir.join("payloads"),
    )
    .whatever("unable to symlink filesystems")?;
    info!("Creating bundle, this may take a while...");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    rugix_bundle::builder::pack(bundle_dir, output).whatever("unable to create bundle")?;
    Ok(())
}

fn rpi_bundle_config(opts: &BundleOpts) -> BundleManifest {
    let compression = if opts.without_compression {
        None
    } else {
        Some(manifest::Compression::Xz(manifest::XzCompression::new()))
    };
    manifest::BundleManifest::new(vec![
        manifest::Payload::new("partition-2.img".to_owned())
            .with_slot(Some("boot".to_owned()))
            .with_block_encoding(Some(
                manifest::BlockEncoding::new(ChunkerAlgorithm::Casync {
                    avg_block_size_kib: 64,
                })
                .with_deduplicate(Some(true))
                .with_compression(compression.clone()),
            )),
        manifest::Payload::new("partition-5.img".to_owned())
            .with_slot(Some("system".to_owned()))
            .with_block_encoding(Some(
                manifest::BlockEncoding::new(ChunkerAlgorithm::Casync {
                    avg_block_size_kib: 64,
                })
                .with_deduplicate(Some(true))
                .with_compression(compression.clone()),
            )),
    ])
}

fn efi_bundle_config(opts: &BundleOpts) -> BundleManifest {
    let compression = if opts.without_compression {
        None
    } else {
        Some(manifest::Compression::Xz(manifest::XzCompression::new()))
    };
    manifest::BundleManifest::new(vec![
        manifest::Payload::new("partition-2.img".to_owned())
            .with_slot(Some("boot".to_owned()))
            .with_block_encoding(Some(
                manifest::BlockEncoding::new(ChunkerAlgorithm::Casync {
                    avg_block_size_kib: 64,
                })
                .with_deduplicate(Some(true))
                .with_compression(compression.clone()),
            )),
        manifest::Payload::new("partition-4.img".to_owned())
            .with_slot(Some("system".to_owned()))
            .with_block_encoding(Some(
                manifest::BlockEncoding::new(ChunkerAlgorithm::Casync {
                    avg_block_size_kib: 64,
                })
                .with_deduplicate(Some(true))
                .with_compression(compression.clone()),
            )),
    ])
}
