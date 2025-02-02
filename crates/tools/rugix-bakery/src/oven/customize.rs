//! Applies a set of recipes to a system.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use reportify::{bail, ResultExt};
use rugix_cli::StatusSegmentRef;
use rugix_common::mount::{MountStack, Mounted};
use tempfile::tempdir;
use tracing::{error, info};
use xscript::{cmd, run, vars, Cmd, ParentEnv, Run};

use crate::cli::status::CliLog;
use crate::config::layers::LayerConfig;
use crate::config::systems::Architecture;
use crate::oven::layer::LayerContext;
use crate::project::layers::Layer;
use crate::project::library::Library;
use crate::project::recipes::{PackageManager, Recipe, StepKind};
use crate::project::repositories::RepositoryIdx;
use crate::project::ProjectRef;
use crate::utils::caching::{mtime, mtime_recursive};
use crate::BakeryResult;

struct Logger {
    cli_log: StatusSegmentRef<CliLog>,
    state: Mutex<LoggerState>,
}

struct LoggerState {
    log_file: fs::File,
    line_buffer: Vec<u8>,
}

impl Logger {
    pub fn new(layer_name: &str, layer_path: &Path) -> BakeryResult<Self> {
        let log_file = fs::File::create(layer_path.join("build.log"))
            .whatever("error creating layer log file")?;
        Ok(Self {
            cli_log: rugix_cli::add_status(CliLog::new(format!("Layer: {layer_name}"))),
            state: Mutex::new(LoggerState {
                log_file,
                line_buffer: Vec::new(),
            }),
        })
    }

    pub fn write(&self, bytes: &[u8]) {
        let mut state = self.state.lock().unwrap();
        let _ = state.log_file.write_all(&bytes);
        for b in bytes {
            if *b == b'\n' {
                self.cli_log
                    .push_line(String::from_utf8_lossy(&state.line_buffer).into_owned());
                state.line_buffer.clear();
            } else {
                state.line_buffer.push(*b);
            }
        }
    }
}

pub fn customize(
    project: &ProjectRef,
    arch: Architecture,
    layer: &Layer,
    src: Option<&Path>,
    target: &Path,
    layer_path: &Path,
) -> BakeryResult<()> {
    let library = project.library()?;
    // Collect the recipes to apply.
    let config = layer.config(arch).unwrap();
    let jobs = recipe_schedule(layer.repo, config, &library)?;
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
        last_modified = last_modified.max(mtime(src).whatever("unable to determine mtime")?);
    }
    let mut force_run = false;
    let used_files = project
        .dir()
        .join(layer_path.join("rebuild-if-changed.txt"));
    if used_files.exists() {
        for line in std::fs::read_to_string(used_files)
            .whatever("unable to read used files")?
            .lines()
        {
            if let Ok(modified) = mtime_recursive(&project.dir().join(line)) {
                last_modified = last_modified.max(modified)
            } else {
                error!("error determining modification time for {line}");
                force_run = true;
            }
        }
    }
    if target.exists()
        && last_modified < mtime(target).whatever("unable to read `mtime` of target")?
        && !force_run
    {
        return Ok(());
    }
    let bundle_dir = tempdir().whatever("unable to create temporary directory")?;
    let bundle_dir = bundle_dir.path();
    if let Some(src) = src {
        info!("Extracting layer.");
        run!(["tar", "-x", "-f", &src, "-C", bundle_dir]).whatever("unable to extract layer")?;
    } else {
        info!("Creating empty layer.");
        std::fs::create_dir_all(&bundle_dir).whatever("unable ot create layer directory")?;
    }
    let layer_ctx = LayerContext {
        project: project.clone(),
        build_dir: bundle_dir.to_path_buf(),
        output_dir: layer_path.to_path_buf(),
    };
    let root_dir = bundle_dir.join("roots/system");
    std::fs::create_dir_all(&root_dir).ok();
    let logger = Logger::new(&layer.name, layer_path)?;
    apply_recipes(&layer_ctx, &logger, project, arch, &jobs, &root_dir)?;
    layer_ctx.extract_artifacts(config.artifacts.as_ref())?;
    info!("packing system files");
    run!(["tar", "-c", "-f", &target, "-C", bundle_dir, "."])
        .whatever("unable to package system files")?;
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
) -> BakeryResult<Vec<RecipeJob>> {
    let mut stack = layer
        .recipes
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|name| library.try_lookup(repo, name))
        .collect::<BakeryResult<Vec<_>>>()?;
    let mut enabled = stack.iter().cloned().collect::<HashSet<_>>();
    while let Some(idx) = stack.pop() {
        let recipe = &library.recipes[idx];
        for name in recipe.config.dependencies.as_deref().unwrap_or_default() {
            let dependency = library.try_lookup(recipe.repository, name)?;
            if enabled.insert(dependency) {
                stack.push(dependency);
            }
        }
    }
    for excluded in layer.exclude.as_deref().unwrap_or_default() {
        let excluded = library.try_lookup(repo, excluded.deref())?;
        enabled.remove(&excluded);
    }
    let parameters = layer
        .parameters
        .as_ref()
        .map(|parameters| {
            parameters
                .iter()
                .map(|(name, parameters)| {
                    let recipe = library.try_lookup(repo, name.deref())?;
                    if !enabled.contains(&recipe) {
                        bail!("recipe with name {name} is not part of the layer");
                    }
                    Ok((recipe, parameters))
                })
                .collect::<BakeryResult<HashMap<_, _>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut recipes = enabled
        .into_iter()
        .map(|idx| {
            let recipe = library.recipes[idx].clone();
            let recipe_params = parameters.get(&idx);
            if let Some(params) = recipe_params {
                for param_name in params.keys() {
                    if !recipe
                        .config
                        .parameters
                        .as_ref()
                        .map(|parameters| parameters.contains_key(param_name))
                        .unwrap_or_default()
                    {
                        bail!(
                            "unknown parameter `{param_name}` of recipe `{}`",
                            recipe.name
                        );
                    }
                }
            }
            let mut parameters = HashMap::new();
            if let Some(p) = &recipe.config.parameters {
                for (name, def) in p {
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
            }
            Ok(RecipeJob { recipe, parameters })
        })
        .collect::<Result<Vec<_>, _>>()?;
    recipes.sort_by_key(|job| -job.recipe.config.priority.unwrap_or_default());
    Ok(recipes)
}

fn run_cmd(logger: &Logger, cmd: Cmd<OsString>) -> BakeryResult<()> {
    let mut command = Command::new(cmd.prog());
    command.args(cmd.args());
    if let Some(vars) = cmd.vars() {
        if vars.is_clean() {
            command.env_clear();
        }
        for (name, value) in vars.values() {
            if let Some(value) = value {
                command.env(name, value);
            } else {
                command.env_remove(name);
            }
        }
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .whatever_with(|_| format!("unable to spawn command {cmd}"))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    fn copy_log<R: Read>(logger: &Logger, mut reader: R) {
        let mut buffer = vec![0; 8192];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            logger.write(&buffer[..read]);
        }
    }

    let status = std::thread::scope(|scope| {
        scope.spawn(|| copy_log(logger, stdout));
        scope.spawn(|| copy_log(logger, stderr));
        child.wait()
    });

    let status = status.whatever_with(|_| format!("unable to spawn command {cmd}"))?;
    if !status.success() {
        bail!("failed with exit code {}", status.code().unwrap_or(1));
    }
    Ok(())
}

fn apply_recipes(
    layer_ctx: &LayerContext,
    logger: &Logger,
    project: &ProjectRef,
    arch: Architecture,
    jobs: &[RecipeJob],
    root_dir_path: &Path,
) -> BakeryResult<()> {
    let mut mount_stack = MountStack::new();

    fn mount_all(
        project: &ProjectRef,
        root_dir_path: &Path,
        stack: &mut MountStack,
    ) -> BakeryResult<()> {
        stack.push(
            Mounted::bind("/dev", root_dir_path.join("dev")).whatever("unable to mount /dev")?,
        );
        stack.push(
            Mounted::bind("/dev/pts", root_dir_path.join("dev/pts"))
                .whatever("unable to mount /dev/pts")?,
        );
        stack.push(
            Mounted::bind("/sys", root_dir_path.join("sys")).whatever("unable to mount /sys")?,
        );
        stack.push(
            Mounted::mount_fs("proc", "proc", root_dir_path.join("proc"))
                .whatever("unable to mount /proc")?,
        );
        stack.push(
            Mounted::mount_fs("tmpfs", "tmpfs", root_dir_path.join("run"))
                .whatever("unable to mount /run")?,
        );
        stack.push(
            Mounted::mount_fs("tmpfs", "tmpfs", root_dir_path.join("tmp"))
                .whatever("unable to mount /tmp")?,
        );

        let project_dir = root_dir_path.join("run/rugix/bakery/project");
        fs::create_dir_all(&project_dir).whatever("unable to create project directory")?;

        let resolved_resolv = root_dir_path.join("run/systemd/resolve/stub-resolv.conf");
        fs::create_dir_all(resolved_resolv.parent().unwrap())
            .whatever("unable to create `systemd/resolve` directory")?;
        let resolv_conf =
            fs::read("/etc/resolv.conf").whatever("unable to read `/etc/resolv.conf")?;
        fs::write(resolved_resolv, resolv_conf).whatever("unable to write `resolv.conf`")?;

        stack.push(
            Mounted::bind(project.dir(), &project_dir)
                .whatever("unable to bind mount project directory")?,
        );

        Ok(())
    }

    let project_dir = root_dir_path.join("run/rugix/bakery/project");

    for (idx, job) in jobs.iter().enumerate() {
        let recipe = &job.recipe;
        info!(
            "[{:>2}/{}] {} {:?}",
            idx + 1,
            jobs.len(),
            recipe
                .config
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
                        ParentEnv
                            .run(cmd.with_vars(vars! {
                                DEBIAN_FRONTEND = "noninteractive"
                            }))
                            .whatever("unable to install packages")?;
                    }
                }
                StepKind::Install => {
                    if mount_stack.is_empty() {
                        mount_all(project, root_dir_path, &mut mount_stack)?;
                    }
                    let bakery_recipe_path = root_dir_path.join("run/rugix/bakery/recipe");
                    fs::create_dir_all(&bakery_recipe_path)
                        .whatever("unable to create recipe directory")?;
                    let _mounted_recipe = Mounted::bind(&recipe.path, &bakery_recipe_path)
                        .whatever("unable to bind mount recipe")?;
                    let chroot_layer_dir = root_dir_path.join("run/rugix/bakery/bundle/");
                    fs::create_dir_all(&chroot_layer_dir)
                        .whatever("unable to create layer bundle directory")?;
                    let _mounted_layer_dir = Mounted::bind(&layer_ctx.build_dir, &chroot_layer_dir)
                        .whatever("unable to bind mount layer bundle")?;
                    let script = format!("/run/rugix/bakery/recipe/steps/{}", step.filename);
                    let mut vars = vars! {
                        DEBIAN_FRONTEND = "noninteractive",
                        RUGIX_BUNDLE_DIR = "/run/rugix/bakery/bundle/",
                        RUGIX_LAYER_DIR = "/run/rugix/bakery/bundle/",
                        RUGIX_ROOT_DIR = "/",
                        RUGIX_PROJECT_DIR = "/run/rugix/bakery/project/",
                        RUGIX_ARCH = arch.as_str(),
                        LAYER_REBUILD_IF_CHANGED = Path::new("/run/rugix/bakery/project").join(&layer_ctx.output_dir).join("rebuild-if-changed.txt"),
                        RECIPE_DIR = "/run/rugix/bakery/recipe/",
                        RECIPE_STEP_PATH = &script,
                    };
                    for (name, value) in &job.parameters {
                        vars.set(format!("RECIPE_PARAM_{}", name.to_uppercase()), value);
                    }
                    run_cmd(
                        logger,
                        Cmd::new("chroot")
                            .add_arg(root_dir_path)
                            .add_arg(&script)
                            .clone()
                            .with_vars(vars),
                    )?;
                }
                StepKind::Run => {
                    let script = recipe.path.join("steps").join(&step.filename);
                    let mut vars = vars! {
                        DEBIAN_FRONTEND = "noninteractive",
                        RUGIX_BUNDLE_DIR = &layer_ctx.build_dir,
                        RUGIX_LAYER_DIR = &layer_ctx.build_dir,
                        RUGIX_ROOT_DIR = root_dir_path,
                        RUGIX_PROJECT_DIR = &project_dir,
                        RUGIX_ARCH = arch.as_str(),
                        LAYER_REBUILD_IF_CHANGED = project_dir.join(&layer_ctx.output_dir).join("rebuild-if-changed.txt"),
                        RECIPE_DIR = &recipe.path,
                        RECIPE_STEP_PATH = &script,
                    };
                    for (name, value) in &job.parameters {
                        vars.set(format!("RECIPE_PARAM_{}", name.to_uppercase()), value);
                    }
                    run_cmd(logger, Cmd::new(&script).with_vars(vars))?;
                }
            }
        }
        layer_ctx.extract_artifacts(recipe.config.artifacts.as_ref())?;
    }

    Ok(())
}
