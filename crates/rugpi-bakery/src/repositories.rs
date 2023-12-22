//! *Repositories* provide recipes, collections, and layers.
//!
//! Generally, a repository is a directory with the following structure:
//!
//! - `rugpi-repository.toml`: Configuration file of the repository (required).
//! - `recipes`: Directory containing recipes (optional).
//! - `collections`: Directory containing collections (optional).
//! - `layers`: Directory containing layers (optional).
//!
//! As an exception, a project's root directory is also treated as a repository, however,
//! in this case, the repository configuration file is not required/used.
//! Instead, the configuration is synthesized from `rugpi-bakery.toml`.
//!
//! ## Sources
//!
//! Repositories can be sourced from different *[sources]*:
//!
//! - [`sources::GitSource`]: Repository sourced from a Git repository.
//! - [`sources::PathSource`]: Repository sourced from a local path.
//!
//! Sources have to be *materialized* into a local directory before they can be used.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};

use self::sources::{MaterializedSource, PathSource, SourceId};
use crate::repositories::sources::Source;

pub mod sources;

/// A collection of repositories.
#[derive(Debug)]
pub struct Repositories {
    /// The repositories of the collection.
    repositories: Vec<Option<Repository>>,
    /// Table for finding repositories by their source.
    source_to_repository: HashMap<SourceId, RepositoryId>,
    /// Path to the project's root directory.
    root_dir: PathBuf,
}

impl Repositories {
    /// Create an empty collection of repositories.
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        let root_dir = root_dir.as_ref();
        Self {
            repositories: Vec::new(),
            source_to_repository: HashMap::new(),
            root_dir: root_dir.to_path_buf(),
        }
    }

    /// Iterator over the loaded repositories.
    pub fn iter(&self) -> impl Iterator<Item = (RepositoryId, &Repository)> {
        self.repositories
            .iter()
            .enumerate()
            .map(|(idx, repository)| {
                (
                    RepositoryId(idx),
                    repository
                        .as_ref()
                        .expect("repository has not been fully loaded yet"),
                )
            })
    }

    /// Load the repository from the project's root directory.
    ///
    /// The *update* flag indicates whether remote repositories should be updated.
    pub fn load_root(
        &mut self,
        repositories: HashMap<String, Source>,
        update: bool,
    ) -> Anyhow<RepositoryId> {
        self.load_repository(
            Source::Path(PathSource { path: "".into() }).materialize(&self.root_dir, update)?,
            RepositoryConfig {
                name: Some("root".to_owned()),
                description: None,
                repositories,
            },
            update,
        )
    }

    /// Load a repository from the given source and return its id.
    ///
    /// The *update* flag indicates whether remote repositories should be updated.
    pub fn load_source(&mut self, source: Source, update: bool) -> Anyhow<RepositoryId> {
        let source_id = source.id();
        if let Some(id) = self.source_to_repository.get(&source_id).cloned() {
            let Some(repository) = &self.repositories[id.0] else {
                bail!("cycle while loading repository from:\n{:?}", source);
            };
            if repository.source.source == source {
                Ok(id)
            } else {
                bail!(
                    "incompatible repository sources:\n{:?}\n{:?}",
                    repository.source.source,
                    source,
                );
            }
        } else {
            let source = source.materialize(&self.root_dir, update)?;
            let config_path = source.dir.join("rugpi-repository.toml");
            let config =
                toml::from_str(&std::fs::read_to_string(&config_path).with_context(|| {
                    format!("reading repository configuration from {config_path:?}")
                })?)?;
            self.load_repository(source, config, update)
        }
    }

    /// Load a repository from an already materialized source and given config.
    fn load_repository(
        &mut self,
        source: MaterializedSource,
        config: RepositoryConfig,
        update: bool,
    ) -> Anyhow<RepositoryId> {
        if self.source_to_repository.contains_key(&source.id) {
            bail!("repository from {} has already been loaded", source.id);
        }
        eprintln!("=> loading repository from source {}", source.id);
        let id = RepositoryId(self.repositories.len());
        self.repositories.push(None);
        self.source_to_repository.insert(source.id.clone(), id);
        let mut repositories = HashMap::new();
        for (name, source) in &config.repositories {
            repositories.insert(name.clone(), self.load_source(source.clone(), update)?);
        }
        let repository = Repository {
            id,
            source,
            config,
            repositories,
        };
        self.repositories[id.0] = Some(repository);
        Ok(id)
    }
}

impl std::ops::Index<RepositoryId> for Repositories {
    type Output = Repository;

    fn index(&self, index: RepositoryId) -> &Self::Output {
        self.repositories[index.0]
            .as_ref()
            .expect("repository has not been fully loaded yet")
    }
}

/// Uniquely identifies a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepositoryId(usize);

/// A repository.
#[derive(Debug)]
pub struct Repository {
    /// The id of the repository.
    pub id: RepositoryId,
    /// The source of the repository.
    pub source: MaterializedSource,
    /// The configuration of the repository.
    pub config: RepositoryConfig,
    /// The repositories used by the repository.
    pub repositories: HashMap<String, RepositoryId>,
}

/// Repository configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// An optional name of the repository.
    pub name: Option<String>,
    /// An optional description of the repository.
    pub description: Option<String>,
    /// The repositories used by the repository.
    #[serde(default)]
    pub repositories: HashMap<String, Source>,
}
