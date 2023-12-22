//! Repository sources, e.g., a local path or a Git repository.

use std::{
    fmt::Display,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use xscript::{read_str, run, LocalEnv, Run};

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
            run!(["git", "clone", &self.url, path.to_string_lossy()])?;
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
