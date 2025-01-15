//! Data structures for representing recipes.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, ops};

use serde::{Deserialize, Serialize};

use reportify::{bail, whatever, ResultExt};

use crate::config::load_config;
use crate::config::recipes::RecipeConfig;
use crate::utils::caching::{mtime_recursive, ModificationTime};
use crate::BakeryResult;

use super::repositories::RepositoryIdx;

/// Auxiliary data structure for loading recipes.
#[derive(Debug)]
pub struct RecipeLoader {
    repository: RepositoryIdx,
    /// Indicates whether the recipe should be included by default.
    default: Option<bool>,
}

impl RecipeLoader {
    /// Constructs a loader with default settings.
    pub fn new(repository: RepositoryIdx) -> Self {
        Self {
            repository,
            default: None,
        }
    }

    /// Sets whether the loaded recipes should be included by default.
    pub fn with_default(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }

    /// Loads a recipe from the given path.
    pub async fn load(&self, path: &Path) -> BakeryResult<Recipe> {
        let path = path.to_path_buf();
        let modified = mtime_recursive(&path).whatever("unable to determine mtime")?;
        let name = path
            .file_name()
            .ok_or_else(|| whatever!("unable to determine recipe name from path `{path:?}`"))?
            .to_string_lossy()
            .into();
        let config_path = path.join("recipe.toml");
        let config = load_config(&config_path).await?;
        let mut steps = Vec::new();
        let steps_dir = path.join("steps");
        if steps_dir.exists() {
            let mut read_dir = tokio::fs::read_dir(&steps_dir)
                .await
                .whatever("unable to read recipe steps")?;
            while let Some(entry) = read_dir
                .next_entry()
                .await
                .whatever("error reading next directory entry")?
            {
                steps.push(RecipeStep::load(&entry.path()).await?);
            }
        }
        steps.sort_by_key(|step| step.position);
        let recipe = Recipe {
            repository: self.repository,
            modified,
            name,
            config,
            steps,
            path,
        };
        Ok(recipe)
    }
}

/// A recipe.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// The lastest modification time of the recipe.
    pub modified: ModificationTime,
    pub repository: RepositoryIdx,
    /// The name of the recipe.
    pub name: RecipeName,
    /// Information about the recipe.
    pub config: RecipeConfig,
    /// The steps of the recipe.
    pub steps: Vec<RecipeStep>,
    /// The path of the recipe.
    pub path: PathBuf,
}

/// A name of a recipe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecipeName(Arc<String>);

impl ops::Deref for RecipeName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: Into<String>> From<T> for RecipeName {
    fn from(value: T) -> Self {
        Self(Arc::new(value.into()))
    }
}

impl fmt::Display for RecipeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A step of a recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    /// The position of the step.
    pub position: u16,
    /// The kind of step.
    pub kind: StepKind,
    /// The filename of the step.
    pub filename: String,
}

impl RecipeStep {
    /// Tries to load a recipe step from the provided path.
    async fn load(path: &Path) -> BakeryResult<Self> {
        let filename = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| whatever!("unable to determine filename of step `{:?}`", path))?
            .to_owned();
        let (position, kind) = filename
            .split_once('-')
            .ok_or_else(|| whatever!("unable to parse filename of step `{:?}`", path))?;
        let position = position.parse().whatever("unable to parse step position")?;
        let kind = match kind.split('.').next().unwrap() {
            "packages" => {
                let packages = tokio::fs::read_to_string(path)
                    .await
                    .whatever("unable to read packages step")?
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
                let manager = match kind.rsplit_once('.') {
                    Some((_, "apt")) => Some(PackageManager::Apt),
                    Some((_, "apk")) => Some(PackageManager::Apk),
                    _ => None,
                };
                StepKind::Packages { packages, manager }
            }
            "install" => StepKind::Install,
            "run" => StepKind::Run,
            _ => bail!("unknown step kind `{kind}`"),
        };
        Ok(Self {
            position,
            kind,
            filename,
        })
    }
}

/// A step kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepKind {
    /// Install the given packages.
    Packages {
        manager: Option<PackageManager>,
        packages: Vec<String>,
    },
    /// Run a script in the `chroot` environment of the system.
    Install,
    /// Run a script on the host machine.
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageManager {
    Apt,
    Apk,
}
