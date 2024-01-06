use std::{collections::HashMap, fs, ops::Deref, sync::Arc};

use rugpi_common::Anyhow;

use super::{
    recipes::{Recipe, RecipeLoader},
    repositories::{ProjectRepositories, RepositoryIdx},
};
use crate::idx_vec::{new_idx_type, IdxVec};

pub struct Library {
    pub repositories: ProjectRepositories,
    pub recipes: IdxVec<RecipeIdx, Arc<Recipe>>,
    pub tables: IdxVec<RepositoryIdx, HashMap<String, RecipeIdx>>,
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
        Ok(Self {
            repositories,
            recipes,
            tables,
        })
    }

    pub fn lookup(&self, repository: RepositoryIdx, name: &str) -> Option<RecipeIdx> {
        if let Some((dependency_name, recipe_name)) = name.split_once('/') {
            let dependency_idx = match dependency_name {
                "rugpi" => self.repositories.core_repository,
                _ => *self.repositories.repositories[repository]
                    .repositories
                    .get(dependency_name)?,
            };
            self.tables[dependency_idx].get(recipe_name).cloned()
        } else {
            self.tables[repository].get(name).cloned()
        }
    }
}

new_idx_type! {
    /// Uniquely identifies a recipe in [`Library`].
    pub RecipeIdx
}
