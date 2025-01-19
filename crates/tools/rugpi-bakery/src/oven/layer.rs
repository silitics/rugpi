use std::path::PathBuf;

use xscript::{run, Run};

use reportify::ResultExt;

use rugix_fs::Copier;

use crate::config::artifacts::{ArtifactDecl, ArtifactsDecl};
use crate::project::ProjectRef;
use crate::BakeryResult;

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
