//! In-memory representation of Rugpi Bakery projects.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use reportify::ResultExt;

use self::config::BakeryConfig;
use self::library::Library;
use self::repositories::ProjectRepositories;
use crate::BakeryResult;

pub mod config;
pub mod images;
pub mod layers;
pub mod library;
pub mod recipes;
pub mod repositories;

/// A project.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Project {
    /// The configuration of the project.
    pub config: BakeryConfig,
    /// The project directory.
    pub dir: PathBuf,
    /// Lazily initialized fields.
    lazy: ProjectLazy,
}

impl Project {
    /// The repositories of the project.
    pub fn repositories(&self) -> BakeryResult<&Arc<ProjectRepositories>> {
        if let Some(repositories) = self.lazy.repositories.get() {
            return Ok(repositories);
        }
        let repositories = ProjectRepositories::load(self)
            .map(Arc::new)
            .whatever("loading repositories")?;
        let _ = self.lazy.repositories.set(repositories);
        Ok(self.lazy.repositories.get().unwrap())
    }

    /// The library of the project.
    pub fn library(&self) -> BakeryResult<&Arc<Library>> {
        if let Some(library) = self.lazy.library.get() {
            return Ok(library);
        }
        let repositories = self.repositories()?.clone();
        let library = Library::load(repositories)
            .map(Arc::new)
            .whatever("loading library")?;
        let _ = self.lazy.library.set(library);
        Ok(self.lazy.library.get().unwrap())
    }
}

/// Lazily initialized fields of [`Project`].
#[derive(Debug, Default, Clone)]
struct ProjectLazy {
    /// The repositories of the project.
    repositories: OnceLock<Arc<ProjectRepositories>>,
    /// The library of the project.
    library: OnceLock<Arc<Library>>,
}

/// Project loader.
#[derive(Debug)]
pub struct ProjectLoader {
    /// The project directory.
    project_dir: PathBuf,
    /// Path to the configuration file.
    config_file: Option<PathBuf>,
}

impl ProjectLoader {
    /// Construct a new project loader with the given project directory.
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
            config_file: None,
        }
    }

    /// Construct a new project loader from the current working directory.
    pub fn current_dir() -> BakeryResult<Self> {
        Ok(Self::new(
            &std::env::current_dir().whatever("unable to determine current directory")?,
        ))
    }

    /// Set the configuration file path relative to the project directory.
    pub fn with_config_file(mut self, config_file: Option<&Path>) -> Self {
        self.config_file = config_file.map(Path::to_path_buf);
        self
    }

    /// The full path to the configuration file.
    fn config_path(&self) -> PathBuf {
        self.project_dir.join(
            self.config_file
                .as_deref()
                .unwrap_or_else(|| Path::new("rugpi-bakery.toml")),
        )
    }

    /// Load the project.
    pub fn load(self) -> BakeryResult<Project> {
        let config = BakeryConfig::load(&self.config_path())?;
        Ok(Project {
            dir: self.project_dir,
            config,
            lazy: ProjectLazy::default(),
        })
    }
}
