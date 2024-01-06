//! In-memory representation of Rugpi Bakery projects.

use std::path::{Path, PathBuf};

use rugpi_common::Anyhow;

use self::config::BakeryConfig;

pub mod config;
pub mod recipes;
pub mod repositories;

/// A project.
#[derive(Debug)]
#[non_exhaustive]
pub struct Project {
    /// The configuration of the project.
    pub config: BakeryConfig,
    /// The project directory.
    pub dir: PathBuf,
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
    pub fn current_dir() -> Anyhow<Self> {
        Ok(Self::new(&std::env::current_dir()?))
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
    pub fn load(self) -> Anyhow<Project> {
        let config = BakeryConfig::load(&self.config_path())?;
        Ok(Project {
            dir: self.project_dir,
            config,
        })
    }
}
