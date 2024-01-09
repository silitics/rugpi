//! Applies a set of recipes to a system.

use std::{
    collections::{HashMap, HashSet},
    fs,
    ops::Deref,
    path::Path,
    sync::Arc,
};

use anyhow::{anyhow, bail};
use clap::Parser;
use rugpi_common::{mount::Mounted, Anyhow};
use tempfile::tempdir;
use xscript::{cmd, run, vars, ParentEnv, Run};

use crate::{
    caching::mtime,
    project::{
        config::Architecture,
        layers::{Layer, LayerConfig},
        library::Library,
        recipes::{Recipe, StepKind},
        Project,
    },
};

/// The arguments of the `customize` command.
#[derive(Debug, Parser)]
pub struct CustomizeTask {
    /// The source archive with the original system.
    src: String,
    /// The destination archive with the modified system.
    dest: String,
}

pub fn customize(
    project: &Project,
    arch: Architecture,
    layer: &Layer,
    src: &Path,
    target: &Path,
) -> Anyhow<()> {
    let library = project.load_library()?;
    // Collect the recipes to apply.
    let config = layer.config(arch).unwrap();
    let jobs = recipe_schedule(config, &library)?;
    let last_modified = jobs
        .iter()
        .map(|job| job.recipe.modified)
        .max()
        .unwrap()
        .max(layer.modified);
    if target.exists() && last_modified < mtime(target)? {
        return Ok(());
    }
    // Prepare system chroot.
    let root_dir = tempdir()?;
    let root_dir_path = root_dir.path();
    println!("Extracting system files...");
    run!(["tar", "-x", "-f", &src, "-C", root_dir_path])?;
    apply_recipes(arch, &jobs, root_dir_path)?;
    println!("Packing system files...");
    run!(["tar", "-c", "-f", &target, "-C", root_dir_path, "."])?;
    Ok(())
}

struct RecipeJob {
    recipe: Arc<Recipe>,
    parameters: HashMap<String, String>,
}

fn recipe_schedule(layer: &LayerConfig, library: &Library) -> Anyhow<Vec<RecipeJob>> {
    let mut stack = layer
        .recipes
        .iter()
        .map(|name| {
            library
                .lookup(library.repositories.root_repository, name.deref())
                .ok_or_else(|| anyhow!("recipe with name {name} not found"))
        })
        .collect::<Anyhow<Vec<_>>>()?;
    let mut enabled = stack.iter().cloned().collect::<HashSet<_>>();
    while let Some(idx) = stack.pop() {
        let recipe = &library.recipes[idx];
        for name in &recipe.info.dependencies {
            let dependency_idx = library
                .lookup(recipe.repository, name.deref())
                .ok_or_else(|| anyhow!("recipe with name {name} not found"))?;
            if enabled.insert(dependency_idx) {
                stack.push(dependency_idx);
            }
        }
    }
    for excluded in &layer.exclude {
        let excluded = library
            .lookup(library.repositories.root_repository, excluded.deref())
            .ok_or_else(|| anyhow!("recipe with name {excluded} not found"))?;
        enabled.remove(&excluded);
    }
    let parameters = layer
        .parameters
        .iter()
        .map(|(name, parameters)| {
            Ok((
                library
                    .lookup(library.repositories.root_repository, name.deref())
                    .ok_or_else(|| anyhow!("recipe with name {name} not found"))?,
                parameters,
            ))
        })
        .collect::<Anyhow<HashMap<_, _>>>()?;
    let mut recipes = enabled
        .into_iter()
        .map(|idx| {
            let recipe = library.recipes[idx].clone();
            let recipe_params = parameters.get(&idx);
            if let Some(params) = recipe_params {
                for param_name in params.keys() {
                    if !recipe.info.parameters.contains_key(param_name) {
                        bail!(
                            "unknown parameter `{param_name}` of recipe `{}`",
                            recipe.name
                        );
                    }
                }
            }
            let mut parameters = HashMap::new();
            for (name, def) in &recipe.info.parameters {
                if let Some(params) = recipe_params {
                    if let Some(value) = params.get(name) {
                        parameters.insert(name.to_owned(), value.to_string());
                        continue;
                    }
                }
                if let Some(default) = &def.default {
                    parameters.insert(name.to_owned(), default.to_string());
                    continue;
                }
                bail!("unable to find value for parameter `{name}`");
            }
            Ok(RecipeJob { recipe, parameters })
        })
        .collect::<Result<Vec<_>, _>>()?;
    // 4️⃣ Sort recipes by priority.
    recipes.sort_by_key(|job| -job.recipe.info.priority);
    Ok(recipes)
}

fn apply_recipes(arch: Architecture, jobs: &Vec<RecipeJob>, root_dir_path: &Path) -> Anyhow<()> {
    let _mounted_dev = Mounted::bind("/dev", root_dir_path.join("dev"))?;
    let _mounted_dev_pts = Mounted::bind("/dev/pts", root_dir_path.join("dev/pts"))?;
    let _mounted_sys = Mounted::bind("/sys", root_dir_path.join("sys"))?;
    let _mounted_proc = Mounted::mount_fs("proc", "proc", root_dir_path.join("proc"))?;
    let _mounted_run = Mounted::mount_fs("tmpfs", "tmpfs", root_dir_path.join("run"))?;
    let _mounted_tmp = Mounted::mount_fs("tmpfs", "tmpfs", root_dir_path.join("tmp"))?;

    let bakery_recipe_path = root_dir_path.join("run/rugpi/bakery/recipe");
    fs::create_dir_all(&bakery_recipe_path)?;

    for (idx, job) in jobs.iter().enumerate() {
        let recipe = &job.recipe;
        println!(
            "[{:>2}/{}] {} {:?}",
            idx + 1,
            jobs.len(),
            recipe
                .info
                .description
                .as_deref()
                .unwrap_or(recipe.name.deref()),
            &job.parameters,
        );
        let _mounted_recipe = Mounted::bind(&recipe.path, &bakery_recipe_path)?;

        for step in &recipe.steps {
            println!("    - {}", step.filename);
            match &step.kind {
                StepKind::Packages { packages } => {
                    let mut cmd = cmd!("chroot", root_dir_path, "apt-get", "install", "-y");
                    cmd.extend_args(packages);
                    ParentEnv.run(cmd.with_vars(vars! {
                        DEBIAN_FRONTEND = "noninteractive"
                    }))?;
                }
                StepKind::Install => {
                    let script = format!("/run/rugpi/bakery/recipe/steps/{}", step.filename);
                    let mut vars = vars! {
                        DEBIAN_FRONTEND = "noninteractive",
                        RUGPI_ROOT_DIR = "/",
                        RUGPI_ARCH = arch.as_str(),
                        RECIPE_DIR = "/run/rugpi/bakery/recipe/",
                        RECIPE_STEP_PATH = &script,
                    };
                    for (name, value) in &job.parameters {
                        vars.set(format!("RECIPE_PARAM_{}", name.to_uppercase()), value);
                    }
                    run!(["chroot", root_dir_path, &script].with_vars(vars))?;
                }
                StepKind::Run => {
                    let script = recipe.path.join("steps").join(&step.filename);
                    let mut vars = vars! {
                        DEBIAN_FRONTEND = "noninteractive",
                        RUGPI_ROOT_DIR = root_dir_path,
                        RUGPI_ARCH = arch.as_str(),
                        RECIPE_DIR = &recipe.path,
                        RECIPE_STEP_PATH = &script,
                    };
                    for (name, value) in &job.parameters {
                        vars.set(format!("RECIPE_PARAM_{}", name.to_uppercase()), value);
                    }
                    run!([&script].with_vars(vars))?;
                }
            }
        }
    }
    Ok(())
}
