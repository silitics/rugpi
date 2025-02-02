use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tracing::info;
use xscript::{run, Run};

use reportify::ResultExt;

use rugix_fs::Copier;

use crate::config::artifacts::{ArtifactDecl, ArtifactsDecl};
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

impl LayerContext {
    pub fn extract_artifacts(&self, artifacts: Option<&ArtifactsDecl>) -> BakeryResult<()> {
        if let Some(artifacts) = artifacts {
            for (name, artifact) in artifacts {
                self.extract_artifact(name, artifact)?;
            }
        }
        Ok(())
    }

    pub fn extract_artifact(&self, name: &str, artifact: &ArtifactDecl) -> BakeryResult<()> {
        let mut output_path = self.project.dir().join(&self.output_dir);
        output_path.push("artifacts");
        std::fs::create_dir_all(&output_path).ok();
        output_path.push(name);
        let mut copier = Copier::new();
        match artifact {
            ArtifactDecl::File(props) => {
                let build_path = self.build_dir.join(&props.path);
                if let Some(extension) = build_path.extension() {
                    output_path.set_extension(extension);
                }
                std::fs::remove_file(&output_path).ok();
                copier
                    .copy_file(&build_path, &output_path)
                    .whatever_with(|_| format!("unable to extract artifact {name}"))?;
            }
            ArtifactDecl::Directory(props) => {
                let build_path = self.build_dir.join(&props.path);
                if props.as_tar.unwrap_or(true) {
                    output_path.set_extension("tar");
                    std::fs::remove_file(&output_path).ok();
                    run!(["tar", "-c", "-f", &output_path, "-C", build_path, "."])
                        .whatever_with(|_| format!("unable to create artifact {name}"))?;
                } else {
                    std::fs::remove_dir_all(&output_path).ok();
                    copier
                        .copy_dir(&build_path, &output_path)
                        .whatever_with(|_| format!("unable to extract artifact {name}"))?;
                }
            }
        }
        Ok(())
    }
}
