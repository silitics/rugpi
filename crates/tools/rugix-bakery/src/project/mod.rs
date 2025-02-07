//! In-memory project representation.

use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use library::Library;
use reportify::ResultExt;
use repositories::ProjectRepositories;
use rugix_hashes::HashAlgorithm;

use crate::config::load_config;
use crate::config::projects::ProjectConfig;
use crate::BakeryResult;

pub mod layers;
pub mod library;
pub mod recipes;
pub mod repositories;

/// Shared reference to an in-memory project.
#[derive(Debug, Clone)]
pub struct ProjectRef {
    /// Shared project state.
    shared: Arc<ProjectShared>,
}

impl ProjectRef {
    /// Project directory.
    pub fn dir(&self) -> &Path {
        &self.shared.dir
    }

    /// Project configuration.
    pub fn config(&self) -> &ProjectConfig {
        &self.shared.config
    }

    /// Retrieve the repositories of the project.
    ///
    /// This may load the repositories lazily.
    pub fn repositories(&self) -> BakeryResult<Arc<ProjectRepositories>> {
        let mut repositories = self.shared.lazy.repositories.lock().unwrap();
        if let Some(repositories) = &*repositories {
            Ok(repositories.clone())
        } else {
            *repositories = Some(ProjectRepositories::load(self).map(Arc::new)?);
            Ok(repositories.clone().unwrap())
        }
    }

    /// Retrieve the library of the project.
    ///
    /// This may load the library lazily.
    pub fn library(&self) -> BakeryResult<Arc<Library>> {
        let mut library = self.shared.lazy.library.lock().unwrap();
        if let Some(library) = &*library {
            Ok(library.clone())
        } else {
            *library = Some(Library::load(self.repositories()?).map(Arc::new)?);
            Ok(library.clone().unwrap())
        }
    }
}

/// Shared project state.
#[derive(Debug)]
struct ProjectShared {
    /// Project directory.
    dir: PathBuf,
    /// Project configuration.
    config: Arc<ProjectConfig>,
    /// Lazily-loaded project data.
    lazy: ProjectLazy,
}

#[derive(Debug, Default)]
struct ProjectLazy {
    repositories: Mutex<Option<Arc<ProjectRepositories>>>,
    library: Mutex<Option<Arc<Library>>>,
}

/// Locally unique id of the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalProjectId(Arc<str>);

/// Project loader.
#[derive(Debug)]
pub struct ProjectLoader {
    /// The project directory.
    project_dir: PathBuf,
    /// Path to the configuration file.
    config_file: Option<PathBuf>,
    /// Local project id.
    local_id: LocalProjectId,
}

impl ProjectLoader {
    /// Construct a new project loader with the given project directory.
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
            config_file: None,
            local_id: local_id_from_bytes(project_dir.as_os_str().as_bytes()),
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

    pub fn with_local_id(mut self, identity: &[u8]) -> Self {
        self.local_id = local_id_from_bytes(identity);
        self
    }

    /// The full path to the configuration file.
    fn config_path(&self) -> PathBuf {
        self.project_dir.join(
            self.config_file
                .as_deref()
                .unwrap_or_else(|| Path::new("rugix-bakery.toml")),
        )
    }

    /// Load the project.
    pub fn load(self) -> BakeryResult<ProjectRef> {
        let config = load_config(&self.config_path())?;
        Ok(ProjectRef {
            shared: Arc::new(ProjectShared {
                dir: self.project_dir,
                config,
                lazy: ProjectLazy::default(),
            }),
        })
    }
}

fn local_id_from_bytes(bytes: &[u8]) -> LocalProjectId {
    LocalProjectId(HashAlgorithm::Sha256.hash(bytes).raw_hex_string()[..10].into())
}
