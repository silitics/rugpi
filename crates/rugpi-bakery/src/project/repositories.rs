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
    fmt::Display,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use xscript::{read_str, run, LocalEnv, Run};

use super::Project;
use crate::idx_vec::{new_idx_type, IdxVec};

#[derive(Debug)]
#[non_exhaustive]
pub struct ProjectRepositories {
    pub repositories: IdxVec<RepositoryIdx, Repository>,
    pub root_repository: RepositoryIdx,
    pub core_repository: RepositoryIdx,
}

impl ProjectRepositories {
    pub fn load(project: &Project) -> Anyhow<Self> {
        let mut repositories = RepositoriesLoader::new(&project.dir);
        let core = repositories.load_source(
            Source::Path(PathSource {
                path: "/usr/share/rugpi/repositories/core".into(),
            }),
            false,
        )?;
        let root = repositories.load_root(project.config.repositories.clone(), true)?;
        Ok(Self {
            repositories: repositories.repositories.map(|_, repo| repo.unwrap()),
            root_repository: root,
            core_repository: core,
        })
    }

    /// Iterator over the loaded repositories.
    pub fn iter(&self) -> impl Iterator<Item = (RepositoryIdx, &Repository)> {
        self.repositories.iter()
    }
}

impl std::ops::Index<RepositoryIdx> for ProjectRepositories {
    type Output = Repository;

    fn index(&self, index: RepositoryIdx) -> &Self::Output {
        &self.repositories[index]
    }
}

/// A collection of repositories.
#[derive(Debug)]
struct RepositoriesLoader {
    /// The repositories of the collection.
    repositories: IdxVec<RepositoryIdx, Option<Repository>>,
    /// Table for finding repositories by their source.
    source_to_repository: HashMap<SourceId, RepositoryIdx>,
    /// Path to the project's root directory.
    root_dir: PathBuf,
}

impl RepositoriesLoader {
    /// Create an empty collection of repositories.
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        let root_dir = root_dir.as_ref();
        Self {
            repositories: IdxVec::new(),
            source_to_repository: HashMap::new(),
            root_dir: root_dir.to_path_buf(),
        }
    }

    /// Load the repository from the project's root directory.
    ///
    /// The *update* flag indicates whether remote repositories should be updated.
    pub fn load_root(
        &mut self,
        repositories: HashMap<String, Source>,
        update: bool,
    ) -> Anyhow<RepositoryIdx> {
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
    pub fn load_source(&mut self, source: Source, update: bool) -> Anyhow<RepositoryIdx> {
        let source_id = source.id();
        if let Some(id) = self.source_to_repository.get(&source_id).cloned() {
            let Some(repository) = &self.repositories[id] else {
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
    ) -> Anyhow<RepositoryIdx> {
        if self.source_to_repository.contains_key(&source.id) {
            bail!("repository from {} has already been loaded", source.id);
        }
        eprintln!("=> loading repository from source {}", source.id);
        let idx = RepositoryIdx(self.repositories.len());
        self.repositories.push(None);
        self.source_to_repository.insert(source.id.clone(), idx);
        let mut repositories = HashMap::new();
        for (name, source) in &config.repositories {
            repositories.insert(name.clone(), self.load_source(source.clone(), update)?);
        }
        let repository = Repository {
            idx,
            source,
            config,
            repositories,
        };
        self.repositories[idx] = Some(repository);
        Ok(idx)
    }
}

new_idx_type! {
    /// An index uniquely identifying a repository in [`Repositories`].
    pub RepositoryIdx
}

/// A repository.
#[derive(Debug)]
pub struct Repository {
    /// The index of the repository.
    pub idx: RepositoryIdx,
    /// The source of the repository.
    pub source: MaterializedSource,
    /// The configuration of the repository.
    pub config: RepositoryConfig,
    /// The repositories used by the repository.
    pub repositories: HashMap<String, RepositoryIdx>,
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

/// A source which has been materialized in a local directory.
#[derive(Debug, Clone)]
pub struct MaterializedSource {
    /// The id of the source.
    pub id: SourceId,
    /// The definition of the source.
    pub source: Source,
    /// The directory where the source has been materialized.
    pub dir: PathBuf,
}

/// Globally unique id of a source.
///
/// The id is computed by hashing the path or URL of a source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceId(Arc<str>);

impl SourceId {
    /// The string representation of the id.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The short string representation of the id.
    pub fn as_short_str(&self) -> &str {
        &self.as_str()[..6]
    }
}

impl Display for SourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The source of a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Source {
    /// The repository is sourced from a local path.
    Path(PathSource),
    /// The repository is sourced from a Git repository.
    Git(GitSource),
}

impl Source {
    /// The globally unique id of the source.
    pub fn id(&self) -> SourceId {
        let mut hasher = Sha1::new();
        match self {
            Source::Path(path_source) => {
                hasher.update(b"path");
                hasher.update(path_source.path.as_os_str().as_bytes());
            }
            Source::Git(git_source) => {
                hasher.update(b"git");
                hasher.update(git_source.url.as_bytes());
                if let Some(inner_path) = &git_source.dir {
                    hasher.update(inner_path.as_os_str().as_bytes());
                }
            }
        }
        SourceId(hex::encode(&hasher.finalize()[..]).into())
    }

    /// Materialize the source within the given project root directory.
    ///
    /// The *update* flag indicates whether remote repositories should be updated.
    pub fn materialize(self, root_dir: &Path, update: bool) -> Anyhow<MaterializedSource> {
        let id = self.id();
        eprintln!("=> materializing source {id}");
        let path = match &self {
            Source::Path(path_source) => root_dir.join(&path_source.path),
            Source::Git(git_source) => {
                let mut path = root_dir.join(".rugpi/repositories");
                path.push(id.as_str());
                git_source.checkout(&path, update)?;
                if let Some(repository_path) = &git_source.dir {
                    path.push(repository_path);
                }
                path
            }
        };
        Ok(MaterializedSource {
            id,
            source: self,
            dir: path,
        })
    }
}

/// A repository sourced from a local path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathSource {
    /// The path relative to the project's root directory.
    pub path: PathBuf,
}

/// A repository sourced from a Git repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSource {
    /// The URL of the Git repository.
    #[serde(rename = "git")]
    pub url: String,
    /// Specifies the branch to use.
    pub branch: Option<String>,
    /// Specifies the tag to use.
    pub tag: Option<String>,
    /// Specifies the revision to use.
    pub rev: Option<String>,
    /// The directory of the repository in the Git repository.
    pub dir: Option<PathBuf>,
}

impl GitSource {
    /// Checkout the Git repository in the given directory.
    ///
    /// The *fetch* flag indicates whether updates should be fetched from the remote.
    fn checkout(&self, path: &Path, fetch: bool) -> Anyhow<()> {
        if !path.exists() {
            run!(["git", "clone", &self.url, path])?;
        }
        let env = LocalEnv::new(path);
        if fetch {
            run!(env, ["git", "fetch", "--all"])?;
        }
        macro_rules! rev_parse {
            ($rev:literal) => {
                read_str!(env, ["git", "rev-parse", "--verify", $rev])
            };
        }
        let mut commit = rev_parse!("refs/remotes/origin/HEAD^{{commit}}")?;
        if let Some(tag) = &self.tag {
            commit = rev_parse!("refs/tags/{tag}^{{commit}}")?;
        }
        if let Some(branch) = &self.branch {
            commit = rev_parse!("refs/remotes/origin/{branch}^{{commit}}")?;
        }
        if let Some(rev) = &self.rev {
            commit = rev_parse!("{rev}^{{commit}}")?;
        }
        let head = rev_parse!("HEAD^{{commit}}")?;
        if head != commit {
            run!(env, ["git", "checkout", commit])?;
        }
        Ok(())
    }
}
