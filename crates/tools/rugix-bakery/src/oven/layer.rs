use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tracing::info;
use xscript::{run, Run};

use reportify::ResultExt;

use crate::project::ProjectRef;
use crate::utils::caching::{mtime, ModificationTime};
use crate::BakeryResult;

#[derive(Debug)]
pub struct FrozenLayer {
    name: String,
    path: PathBuf,
}

impl FrozenLayer {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }

    pub fn last_modified(&self) -> BakeryResult<ModificationTime> {
        mtime(&self.path).whatever_with(|_| {
            format!(
                "unable to determine modification time of layer {}",
                self.name
            )
        })
    }

    pub fn unfreeze(&self) -> BakeryResult<Layer> {
        let tempdir = TempDir::new().whatever("unable to create temporary directory")?;
        info!("Extracting layer.");
        run!(["tar", "-xf", &self.path, "-C", tempdir.path()])
            .whatever_with(|_| format!("unable to extract layer {}", self.name))?;
        Ok(Layer {
            name: self.name.clone(),
            tempdir,
        })
    }
}

#[derive(Debug)]
pub struct Layer {
    name: String,
    tempdir: TempDir,
}

impl Layer {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }
}

impl AsRef<Path> for Layer {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

pub struct LayerContext {
    pub project: ProjectRef,
    pub build_dir: PathBuf,
    pub output_dir: PathBuf,
}
