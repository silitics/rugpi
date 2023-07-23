//! Data structures for representing recipes.

use std::{collections::HashMap, ffi::OsStr, fmt, fs, ops, path::Path, sync::Arc};

use anyhow::{anyhow, bail, Context};
use camino::Utf8PathBuf;
use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};

/// A library of recipes.
#[derive(Debug, Clone, Default)]
pub struct RecipeLibrary(HashMap<RecipeName, Arc<Recipe>>);

impl RecipeLibrary {
    /// Constructs an empty recipe library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a recipe to the library.
    pub fn add(&mut self, recipe: Recipe) -> Result<(), Recipe> {
        if self.0.contains_key(&recipe.name) {
            Err(recipe)
        } else {
            self.0.insert(recipe.name.clone(), Arc::new(recipe));
            Ok(())
        }
    }

    /// Retrieves a recipe from the library.
    pub fn get(&self, name: &RecipeName) -> Anyhow<&Arc<Recipe>> {
        self.0
            .get(name)
            .ok_or_else(|| anyhow!("recipe with name `{name}` does not exist"))
    }

    /// Returns a loader for the library.
    pub fn loader(&mut self) -> RecipeLoader {
        RecipeLoader::new(self)
    }
}

impl<'lib> IntoIterator for &'lib RecipeLibrary {
    type Item = (&'lib RecipeName, &'lib Arc<Recipe>);

    type IntoIter = std::collections::hash_map::Iter<'lib, RecipeName, Arc<Recipe>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Auxiliary data structure for loading recipes.
#[derive(Debug)]
pub struct RecipeLoader<'lib> {
    /// The library into which recipes shall be loaded.
    library: &'lib mut RecipeLibrary,
    /// Indicates whether the recipe should be included by default.
    default: Option<bool>,
}

impl<'lib> RecipeLoader<'lib> {
    /// Constructs a loader with default settings.
    fn new(library: &'lib mut RecipeLibrary) -> Self {
        Self {
            library,
            default: None,
        }
    }

    /// Sets whether the loaded recipes should be included by default.
    pub fn with_default(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }

    /// Loads a recipe from the given path.
    pub fn load(&mut self, path: &Path) -> Anyhow<()> {
        let path = Utf8PathBuf::from_path_buf(path.to_path_buf())
            .map_err(|_| anyhow!("recipe path must be valid UTF-8"))?;
        let name = path
            .file_name()
            .ok_or_else(|| anyhow!("unable to determine recipe name from path `{path:?}`"))?
            .into();
        let info_path = path.join("recipe.toml");
        let info = toml::from_str(
            &fs::read_to_string(&info_path)
                .with_context(|| format!("error reading recipe info from path `{info_path:?}"))?,
        )
        .with_context(|| format!("error parsing recipe info from path `{info_path:?}`"))?;
        let mut steps = Vec::new();
        let steps_dir = path.join("steps");
        if steps_dir.exists() {
            for entry in fs::read_dir(&steps_dir)? {
                steps.push(RecipeStep::load(&entry?.path())?);
            }
        }
        steps.sort_by_key(|step| step.position);
        let mut recipe = Recipe {
            name,
            info,
            steps,
            path,
        };
        if let Some(default) = self.default {
            recipe.info.default.get_or_insert(default);
        }
        self.library
            .add(recipe)
            .map_err(|recipe| anyhow!("a recipe with the name `{}` already exists", recipe.name))?;
        Ok(())
    }

    /// Loads all recipes from the given path.
    pub fn load_all(&mut self, path: &Path) -> Anyhow<()> {
        for entry in fs::read_dir(path)? {
            self.load(&entry?.path())?;
        }
        Ok(())
    }
}

/// A recipe.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// The name of the recipe.
    pub name: RecipeName,
    /// Information about the recipe.
    pub info: RecipeInfo,
    /// The steps of the recipe.
    pub steps: Vec<RecipeStep>,
    /// The path of the recipe.
    pub path: Utf8PathBuf,
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

/// Information about a recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeInfo {
    /// An optional description of the recipe.
    pub description: Option<String>,
    /// Indicates whether the recipe should be included by default.
    pub default: Option<bool>,
    /// The priority of the recipe.
    #[serde(default)]
    pub priority: i64,
    /// The dependencies of the recipe.
    #[serde(default)]
    pub dependencies: Vec<RecipeName>,
    /// The parameters of the recipe.
    #[serde(default)]
    pub parameters: HashMap<String, ParameterDef>,
}

/// Definition of a recipe parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterDef {
    /// The default value of the parameter.
    pub default: Option<ParameterValue>,
}

/// A value of a parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    /// A string.
    String(String),
    /// A boolean.
    Boolean(bool),
    /// An integer.
    Integer(i64),
    /// A floating-point number.
    Float(f64),
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValue::String(value) => value.fmt(f),
            ParameterValue::Boolean(value) => value.fmt(f),
            ParameterValue::Integer(value) => value.fmt(f),
            ParameterValue::Float(value) => value.fmt(f),
        }
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
    fn load(path: &Path) -> Anyhow<Self> {
        let filename = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| anyhow!("unable to determine filename of step `{:?}`", path))?
            .to_owned();
        let (position, kind) = filename
            .split_once("-")
            .ok_or_else(|| anyhow!("unable to parse filename of step `{:?}`", path))?;
        let position = position.parse().context("unable to parse step position")?;
        let kind = match kind.split(".").next().unwrap() {
            "packages" => {
                let packages = fs::read_to_string(path)?
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
                StepKind::Packages { packages }
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
    Packages { packages: Vec<String> },
    /// Run a script in the `chroot` environment of the system.
    Install,
    /// Run a script on the host machine.
    Run,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rugpi_common::Anyhow;

    use crate::recipes::RecipeLibrary;

    #[test]
    pub fn test_load_builtin_library() -> Anyhow<()> {
        let mut library = RecipeLibrary::new();
        library
            .loader()
            .load_all(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../recipes"))
    }
}
