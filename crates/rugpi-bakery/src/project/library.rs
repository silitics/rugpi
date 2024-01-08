use std::{collections::HashMap, fs, ops::Deref, sync::Arc};

use rugpi_common::Anyhow;

use super::{
    layers::LayerConfig,
    recipes::{Recipe, RecipeLoader},
    repositories::{ProjectRepositories, RepositoryIdx},
};
use crate::idx_vec::{new_idx_type, IdxVec};

pub struct Library {
    pub repositories: ProjectRepositories,
    pub recipes: IdxVec<RecipeIdx, Arc<Recipe>>,
    pub layers: IdxVec<LayerIdx, LayerConfig>,
    pub recipe_tables: IdxVec<RepositoryIdx, HashMap<String, RecipeIdx>>,
    pub layer_tables: IdxVec<RepositoryIdx, HashMap<String, LayerIdx>>,
}

impl Library {
    pub fn load(repositories: ProjectRepositories) -> Anyhow<Self> {
        let mut recipes = IdxVec::new();
        let tables = IdxVec::<RepositoryIdx, _>::from_vec(
            repositories
                .repositories
                .iter()
                .map(|(idx, repository)| -> Anyhow<_> {
                    let mut table = HashMap::new();
                    let loader =
                        RecipeLoader::new(idx).with_default(idx == repositories.root_repository);
                    for entry in fs::read_dir(repository.source.dir.join("recipes"))? {
                        let entry = entry?;
                        let recipe = loader.load(&entry.path())?;
                        let recipe_idx = recipes.push(Arc::new(recipe));
                        table.insert(recipes[recipe_idx].name.deref().to_owned(), recipe_idx);
                    }
                    Ok(table)
                })
                .collect::<Anyhow<_>>()?,
        );
        let mut layers = IdxVec::new();
        let layer_tables = IdxVec::<RepositoryIdx, _>::from_vec(
            repositories
                .repositories
                .iter()
                .map(|(_, repository)| -> Anyhow<_> {
                    let mut table = HashMap::new();
                    let layers_dir = repository.source.dir.join("layers");
                    if !layers_dir.exists() {
                        return Ok(table);
                    }
                    for entry in fs::read_dir(layers_dir)? {
                        let entry = entry?;
                        let path = entry.path();
                        let layer_name = path.file_stem().unwrap();
                        let layer_config = LayerConfig::load(&path)?;
                        let layer_idx = layers.push(layer_config);
                        table.insert(layer_name.to_string_lossy().into_owned(), layer_idx);
                    }
                    Ok(table)
                })
                .collect::<Anyhow<_>>()?,
        );
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
