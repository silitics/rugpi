//! Applies a set of recipes to a system.

use std::{
    collections::{HashMap, HashSet},
    fs,
    ops::Deref,
    path::Path,
    sync::Arc,
};

use rugpi_common::mount::{MountStack, Mounted};
use tempfile::tempdir;
use xscript::{cmd, run, vars, ParentEnv, Run};

use crate::{
    project::{
        config::Architecture,
        layers::{Layer, LayerConfig},
        library::Library,
        recipes::{PackageManager, Recipe, StepKind},
        repositories::RepositoryIdx,
        Project,
    },
    utils::{
        caching::{mtime, mtime_recursive},
        prelude::*,
    },
};

pub fn customize(
    project: &Project,
    arch: Architecture,
    layer: &Layer,
    src: Option<&Path>,
    target: &Path,
    layer_path: &Path,
) -> Anyhow<()> {
    let library = project.library()?;
    // Collect the recipes to apply.
    let config = layer.config(arch).unwrap();
    let jobs = recipe_schedule(layer.repo, config, library)?;
    if jobs.is_empty() {
        bail!("layer must have recipes")
    }
    let mut last_modified = jobs
        .iter()
        .map(|job| job.recipe.modified)
        .max()
        .unwrap()
        .max(layer.modified);
    if let Some(src) = src {
        last_modified = last_modified.max(mtime(src)?);
    }
    let mut force_run = false;
    let used_files = project.dir.join(layer_path.join("rebuild-if-changed.txt"));
    if used_files.exists() {
        for line in std::fs::read_to_string(used_files)?.lines() {
            if let Ok(modified) = mtime_recursive(&project.dir.join(line)) {
                last_modified = last_modified.max(modified)
            } else {
                error!("error determining modification time for {line}");
                force_run = true;
            }
        }
    }
    if target.exists() && last_modified < mtime(target)? && !force_run {
        return Ok(());
    }
    let bundle_dir = tempdir()?;
    let bundle_dir = bundle_dir.path();
    if let Some(src) = src {
        info!("Extracting layer.");
        run!(["tar", "-x", "-f", &src, "-C", bundle_dir])?;
    } else {
        info!("Creating empty layer.");
        std::fs::create_dir_all(&bundle_dir)?;
    }
    let root_dir = bundle_dir.join("roots/system");
    std::fs::create_dir_all(&root_dir).ok();
    apply_recipes(project, arch, &jobs, bundle_dir, &root_dir, layer_path)?;
    info!("packing system files");
    run!(["tar", "-c", "-f", &target, "-C", bundle_dir, "."])?;
    Ok(())
}

struct RecipeJob {
    recipe: Arc<Recipe>,
    parameters: HashMap<String, String>,
}

fn recipe_schedule(
    repo: RepositoryIdx,
    layer: &LayerConfig,
    library: &Library,
) -> Anyhow<Vec<RecipeJob>> {
    let mut stack = layer
        .recipes
        .iter()
        .map(|name| library.try_lookup(repo, name.deref()))
        .collect::<Anyhow<Vec<_>>>()?;
    let mut enabled = stack.iter().cloned().collect::<HashSet<_>>();
    while let Some(idx) = stack.pop() {
        let recipe = &library.recipes[idx];
        for name in &recipe.info.dependencies {
            let dependency = library.try_lookup(recipe.repository, name.deref())?;
            if enabled.insert(dependency) {
                stack.push(dependency);
            }
        }
    }
    for excluded in &layer.exclude {
        let excluded = library.try_lookup(repo, excluded.deref())?;
        enabled.remove(&excluded);
    }
    let parameters = layer
        .parameters
        .iter()
        .map(|(name, parameters)| {
            let recipe = library.try_lookup(repo, name.deref())?;
            if !enabled.contains(&recipe) {
                bail!("recipe with name {name} is not part of the layer");
            }
            Ok((recipe, parameters))
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
    recipes.sort_by_key(|job| -job.recipe.info.priority);
    Ok(recipes)
}

fn apply_recipes(
    project: &Project,
    arch: Architecture,
    jobs: &[RecipeJob],
    bundle_dir: &Path,
    root_dir_path: &Path,
    layer_path: &Path,
) -> Anyhow<()> {
    let mut mount_stack = MountStack::new();

    fn mount_all(project: &Project, root_dir_path: &Path, stack: &mut MountStack) -> Anyhow<()> {
        stack.push(Mounted::bind("/dev", root_dir_path.join("dev"))?);
        stack.push(Mounted::bind("/dev/pts", root_dir_path.join("dev/pts"))?);
        stack.push(Mounted::bind("/sys", root_dir_path.join("sys"))?);
        stack.push(Mounted::mount_fs(
            "proc",
            "proc",
            root_dir_path.join("proc"),
        )?);
        stack.push(Mounted::mount_fs(
            "tmpfs",
            "tmpfs",
            root_dir_path.join("run"),
        )?);
        stack.push(Mounted::mount_fs(
            "tmpfs",
            "tmpfs",
            root_dir_path.join("tmp"),
        )?);

        let project_dir = root_dir_path.join("run/rugpi/bakery/project");
        fs::create_dir_all(&project_dir)?;

        stack.push(Mounted::bind(&project.dir, &project_dir)?);

        Ok(())
    }

    let project_dir = root_dir_path.join("run/rugpi/bakery/project");

    for (idx, job) in jobs.iter().enumerate() {
        let recipe = &job.recipe;
        info!(
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

        for step in &recipe.steps {
            info!("    - {}", step.filename);
            match &step.kind {
                StepKind::Packages { packages, manager } => {
                    if mount_stack.is_empty() {
                        mount_all(project, root_dir_path, &mut mount_stack)?;
                    }
                    let chroot_manager = if root_dir_path.join("usr/bin/apt-get").exists() {
                        PackageManager::Apt
                    } else if root_dir_path.join("sbin/apk").exists() {
                        PackageManager::Apk
                    } else {
                        bail!("unable to determine package manager")
                    };
                    let manager = manager.unwrap_or(chroot_manager);
                    if manager == chroot_manager {
                        let mut cmd = match manager {
                            PackageManager::Apt => {
                                cmd!("chroot", root_dir_path, "apt-get", "install", "-y")
                            }
                            PackageManager::Apk => {
                                cmd!("chroot", root_dir_path, "apk", "add", "--no-interactive")
                            }
                        };
                        cmd.extend_args(packages);
                        ParentEnv.run(cmd.with_vars(vars! {
                            DEBIAN_FRONTEND = "noninteractive"
                        }))?;
                    }
                }
                StepKind::Install => {
                    if mount_stack.is_empty() {
                        mount_all(project, root_dir_path, &mut mount_stack)?;
                    }
                    let bakery_recipe_path = root_dir_path.join("run/rugpi/bakery/recipe");
                    fs::create_dir_all(&bakery_recipe_path)?;
                    let _mounted_recipe = Mounted::bind(&recipe.path, &bakery_recipe_path)?;
                    let chroot_layer_dir = root_dir_path.join("run/rugpi/bakery/bundle/");
                    fs::create_dir_all(&chroot_layer_dir)?;
                    let _mounted_layer_dir = Mounted::bind(&bundle_dir, &chroot_layer_dir)?;
                    let script = format!("/run/rugpi/bakery/recipe/steps/{}", step.filename);
                    let mut vars = vars! {
                        DEBIAN_FRONTEND = "noninteractive",
                        RUGPI_BUNDLE_DIR = "/run/rugpi/bakery/bundle/",
                        RUGPI_ROOT_DIR = "/",
                        RUGPI_PROJECT_DIR = "/run/rugpi/bakery/project/",
                        RUGPI_ARCH = arch.as_str(),
                        LAYER_REBUILD_IF_CHANGED = Path::new("/run/rugpi/bakery/project").join(layer_path).join("rebuild-if-changed.txt"),
                        RECIPE_DIR = "/run/rugpi/bakery/recipe/",
                        RECIPE_STEP_PATH = &script,
                    };
                    for (name, value) in &job.parameters {
                        vars.set(format!("RECIPE_PARAM_{}", name.to_uppercase()), value);
                    }
                    run!(["chroot", root_dir_path, &script]
                        .with_stdout(xscript::Out::Inherit)
                        .with_stderr(xscript::Out::Inherit)
                        .with_vars(vars))?;
                }
                StepKind::Run => {
                    let script = recipe.path.join("steps").join(&step.filename);
                    let mut vars = vars! {
                        DEBIAN_FRONTEND = "noninteractive",
                        RUGPI_BUNDLE_DIR = bundle_dir,
                        RUGPI_ROOT_DIR = root_dir_path,
                        RUGPI_PROJECT_DIR = &project_dir,
                        RUGPI_ARCH = arch.as_str(),
                        LAYER_REBUILD_IF_CHANGED = project_dir.join(layer_path).join("rebuild-if-changed.txt"),
                        RECIPE_DIR = &recipe.path,
                        RECIPE_STEP_PATH = &script,
                    };
                    for (name, value) in &job.parameters {
                        vars.set(format!("RECIPE_PARAM_{}", name.to_uppercase()), value);
                    }
                    run!([&script]
                        .with_stdout(xscript::Out::Inherit)
                        .with_stderr(xscript::Out::Inherit)
                        .with_vars(vars))?;
                }
            }
        }
    }

    Ok(())
}
