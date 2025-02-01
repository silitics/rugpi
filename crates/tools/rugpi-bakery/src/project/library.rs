use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use reportify::{whatever, ResultExt};

use crate::config::load_config;
use crate::config::systems::Architecture;
use crate::utils::caching::mtime;
use crate::utils::idx_vec::{new_idx_type, IdxVec};
use crate::BakeryResult;

use super::layers::Layer;
use super::recipes::{Recipe, RecipeLoader};
use super::repositories::{ProjectRepositories, RepositoryIdx};

#[derive(Debug)]
pub struct Library {
    pub repositories: Arc<ProjectRepositories>,
    pub recipes: IdxVec<RecipeIdx, Arc<Recipe>>,
    pub layers: IdxVec<LayerIdx, Layer>,
    pub recipe_tables: IdxVec<RepositoryIdx, HashMap<String, RecipeIdx>>,
    pub layer_tables: IdxVec<RepositoryIdx, HashMap<String, LayerIdx>>,
}

impl Library {
    #[allow(clippy::assigning_clones)]
    pub fn load(repositories: Arc<ProjectRepositories>) -> BakeryResult<Self> {
        let mut recipes = IdxVec::new();
        let mut tables = IdxVec::<RepositoryIdx, _>::new();
        for (idx, repository) in repositories.iter() {
            let mut table = HashMap::new();
            let loader = RecipeLoader::new(idx).with_default(idx == repositories.root_repository);
            let recipes_dir = repository.source.dir.join("recipes");
            if recipes_dir.is_dir() {
                for entry in fs::read_dir(repository.source.dir.join("recipes"))
                    .whatever("error reading recipes from directory")?
                {
                    let entry = entry.whatever("error reading recipe directory entry")?;
                    let path = entry.path();
                    if !path.is_dir() || should_ignore_path(&path) {
                        continue;
                    }
                    let recipe = loader.load(&entry.path())?;
                    let recipe_idx = recipes.push(Arc::new(recipe));
                    table.insert(recipes[recipe_idx].name.deref().to_owned(), recipe_idx);
                }
            }
            tables.push(table);
        }
        let mut layers = IdxVec::new();
        let mut layer_tables = IdxVec::<RepositoryIdx, _>::new();
        for (idx, repository) in repositories.iter() {
            let mut table = HashMap::new();
            let layers_dir = repository.source.dir.join("layers");
            if !layers_dir.exists() {
                layer_tables.push(table);
                continue;
            }
            for entry in
                fs::read_dir(layers_dir).whatever("unable to read layers from directory")?
            {
                let entry = entry.whatever("unable to read layer directory entry")?;
                let path = entry.path();
                if !path.is_file() || should_ignore_path(&path) {
                    continue;
                }
                if path.extension() != Some(OsStr::new("toml")) {
                    continue;
                }
                let mut name = path.file_stem().unwrap().to_string_lossy().into_owned();
                let mut arch = None;
                if let Some((layer_name, arch_str)) = name.split_once('.') {
                    arch = Some(
                        Architecture::from_str(arch_str)
                            .whatever("unable to parse architecture")?,
                    );
                    name = layer_name.to_owned();
                }
                let modified = mtime(&path).whatever("unable to obtain layer mtime")?;
                let layer_config = load_config(&path)?;
                let layer_idx = *table
                    .entry(name.clone())
                    .or_insert_with(|| layers.push(Layer::new(name, idx, modified)));
                layers[layer_idx].modified = layers[layer_idx].modified.max(modified);
                match arch {
                    Some(arch) => {
                        layers[layer_idx].arch_configs.insert(arch, layer_config);
                    }
                    None => {
                        layers[layer_idx].default_config = Some(layer_config);
                    }
                }
            }
            layer_tables.push(table);
        }
        Ok(Self {
            repositories,
            recipes,
            recipe_tables: tables,
            layers,
            layer_tables,
        })
    }

    pub fn lookup(&self, repository: RepositoryIdx, name: &str) -> Option<RecipeIdx> {
        if let Some((dependency_name, recipe_name)) = name.split_once('/') {
            let dependency_idx = match dependency_name {
                "core" => self.repositories.core_repository,
                _ => *self.repositories.repositories[repository]
                    .repositories
                    .get(dependency_name)?,
            };
            self.recipe_tables[dependency_idx].get(recipe_name).cloned()
        } else {
            self.recipe_tables[repository].get(name).cloned()
        }
    }

    pub fn try_lookup(&self, repo: RepositoryIdx, name: &str) -> BakeryResult<RecipeIdx> {
        self.lookup(repo, name)
            .ok_or_else(|| whatever!("unable to find recipe {name}"))
    }

    pub fn lookup_layer(&self, repo: RepositoryIdx, name: &str) -> Option<LayerIdx> {
        if let Some((dependency_name, layer_name)) = name.split_once('/') {
            let dependency_idx = match dependency_name {
                "core" => self.repositories.core_repository,
                _ => *self.repositories.repositories[repo]
                    .repositories
                    .get(dependency_name)?,
            };
            self.layer_tables[dependency_idx].get(layer_name).cloned()
        } else {
            self.layer_tables[repo].get(name).cloned()
        }
    }
}

new_idx_type! {
    /// Uniquely identifies a recipe in [`Library`].
    pub RecipeIdx
}

new_idx_type! {
    /// Uniquely identifies a layer in [`Library`].
    pub LayerIdx
}

/// Indicates whether the given path should be ignored when scanning for recipes and
/// layers.
fn should_ignore_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };
    matches!(&*file_name.to_string_lossy(), ".DS_Store")
}
