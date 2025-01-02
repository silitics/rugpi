use std::collections::HashMap;
use std::convert::Infallible;
use std::ffi::{CStr, CString, OsStr};
use std::fs;
use std::path::{Path, PathBuf};

use bake::image::make_image;
use bake::LayerBakery;
use clap::Parser;
use colored::Colorize;
use project::config::Architecture;
use project::images::ImageConfig;
use project::repositories::Source;
use project::{Project, ProjectLoader};
use reportify::{bail, Report, ResultExt};
use rugpi_common::fsutils::copy_recursive;
use serde::Deserialize;
use test::RugpiTestError;

pub mod bake;
pub mod project;
pub mod test;
pub mod utils;

reportify::new_whatever_type! {
    BakeryError
}

pub type BakeryResult<T> = Result<T, Report<BakeryError>>;

/// Command line arguments.
#[derive(Debug, Parser)]
#[command(author, about = None, long_about = None)]
pub struct Args {
    /// Path to the `rugpi-bakery.toml` configuration file.
    #[clap(long)]
    config: Option<PathBuf>,
    /// The command to execute.
    #[clap(subcommand)]
    command: Command,
}

/// Commands of the CLI.
#[derive(Debug, Parser)]
pub enum Command {
    /// Bake an image or a layer.
    #[clap(subcommand)]
    Bake(BakeCommand),
    /// List images, recipes, and layers.
    #[clap(subcommand)]
    List(ListCommand),
    /// Spawn a shell in the Rugpi Bakery Docker container.
    Shell,
    /// Pull in external repositories.
    Pull,
    /// Update Rugpi Bakery itself.
    Update(UpdateCommand),
    /// Initialize the project.
    Init(InitCommand),
    /// Run integration tests.
    Test(TestCommand),
    /// Internal unstable commands.
    #[clap(subcommand)]
    Internal(InternalCommand),
}

/// The `list` command.
#[derive(Debug, Parser)]
pub enum ListCommand {
    /// List available images.
    Images,
}

/// The `bake` command.
#[derive(Debug, Parser)]
pub enum BakeCommand {
    /// Bake an image.
    Image {
        /// The name of the image to bake.
        image: String,
        /// The output path of the resulting image.
        output: Option<PathBuf>,
    },
    /// Bake a layer.
    Layer {
        /// The architecture to bake the layer for.
        #[clap(long)]
        arch: Architecture,
        /// The name of the layer to bake.
        layer: String,
    },
}

/// The `test` command.
#[derive(Debug, Parser)]
pub struct TestCommand {
    workflows: Vec<String>,
}

/// The `bake` command.
#[derive(Debug, Parser)]
pub enum InternalCommand {
    MakeImage {
        config: PathBuf,
        source: PathBuf,
        image: PathBuf,
    },
}

/// The `update` command.
#[derive(Debug, Parser)]
pub struct UpdateCommand {
    /// The version to update to.
    version: Option<String>,
}

/// The `init` command.
#[derive(Debug, Parser)]
pub struct InitCommand {
    /// Template to use.
    template: Option<String>,
}

/// Entrypoint of the CLI.
fn main() -> BakeryResult<()> {
    rugpi_cli::Initializer::new().init();

    let args = Args::parse();
    match &args.command {
        Command::Bake(command) => {
            let project = load_project(&args)?;
            match command {
                BakeCommand::Image { image, output } => {
                    let output = output.clone().unwrap_or_else(|| {
                        Path::new("build/images").join(image).with_extension("img")
                    });
                    bake::bake_image(&project, image, &output)?;
                }
                BakeCommand::Layer { layer, arch } => {
                    LayerBakery::new(&project, *arch).bake_root(layer)?;
                }
            }
        }
        Command::Shell => {
            exec_shell()?;
        }
        Command::Update(task) => {
            let version = task.version.as_deref().unwrap_or("v0");
            println!("Switch Rugpi Bakery to version `{version}`...");
            std::fs::write("run-bakery", interpolate_run_bakery(version))
                .whatever("error writing `run-bakery`")?;
        }
        Command::Pull => {
            let project = load_project(&args)?;
            for (_, repository) in project.repositories()?.iter() {
                println!(
                    "{} {} {}",
                    repository.source.id.as_short_str().blue(),
                    repository.config.name.as_deref().unwrap_or("<unknown>"),
                    repository
                        .config
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .bright_black(),
                );
                match &repository.source.source {
                    Source::Path(path_source) => {
                        println!(
                            "  {}{}",
                            "source path ./".bright_black(),
                            path_source.path.to_string_lossy().bright_black()
                        );
                    }
                    Source::Git(git_source) => {
                        println!(
                            "  {}{}",
                            "source git ".bright_black(),
                            git_source.url.bright_black()
                        );
                    }
                }
            }
        }
        Command::Internal(cmd) => match cmd {
            InternalCommand::MakeImage {
                config,
                source,
                image,
            } => {
                let config: ImageConfig = toml::from_str(
                    &fs::read_to_string(&config).whatever("error reading image config")?,
                )
                .whatever("error parsing image config")?;
                make_image(&config, source, image)?;
            }
        },
        Command::Init(cmd) => {
            let Some(template) = &cmd.template else {
                let templates: HashMap<String, TemplateInfo> = toml::from_str(
                    &std::fs::read_to_string("/usr/share/rugpi/templates/templates.toml")
                        .whatever("error reading templates list")?,
                )
                .whatever("error parsing templates list")?;
                println!("{}\n", "Available Templates:".bold());
                let mut names = templates.keys().collect::<Vec<_>>();
                names.sort();
                for name in names {
                    let info = templates.get(name).unwrap();
                    println!(
                        "  {}:\n    {}",
                        name.blue(),
                        info.description.trim().bright_black()
                    );
                }
                return Ok(());
            };
            if Path::new("rugpi-bakery.toml").exists() {
                bail!("Project has already been initialized.");
            }
            let template_dir = Path::new("/usr/share/rugpi/templates").join(template);
            copy_recursive(template_dir, "/project")
                .whatever("error copying template to project directory")?;
        }
        Command::List(cmd) => {
            let project = load_project(&args)?;
            match cmd {
                ListCommand::Images => {
                    println!("Available Images:");
                    for name in project.config.images.keys() {
                        println!("  {name}");
                    }
                }
            }
        }
        Command::Test(test_command) => {
            let project = load_project(&args)?;
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let mut workflows = Vec::new();
                    if test_command.workflows.is_empty() {
                        let mut read_dir = tokio::fs::read_dir(project.dir.join("tests"))
                            .await
                            .whatever("unable to scan for test workflows")?;
                        while let Some(entry) = read_dir
                            .next_entry()
                            .await
                            .whatever("unable to read entry")?
                        {
                            let path = entry.path();
                            if path.extension() == Some(OsStr::new("toml")) {
                                workflows.push(path);
                            }
                        }
                    } else {
                        for name in &test_command.workflows {
                            workflows
                                .push(project.dir.join("tests").join(name).with_extension("toml"));
                        }
                    };
                    for workflow in workflows {
                        test::main(&project, &workflow).await?;
                    }
                    <Result<(), Report<RugpiTestError>>>::Ok(())
                })
                .whatever("unable to run test")?;
        }
    }
    Ok(())
}

fn interpolate_run_bakery(version: &str) -> String {
    include_str!("../assets/run-bakery").replace("%%DEFAULT_VERSION%%", version)
}

fn load_project(args: &Args) -> BakeryResult<Project> {
    ProjectLoader::current_dir()?
        .with_config_file(args.config.as_deref())
        .load()
}

fn exec_shell() -> BakeryResult<Infallible> {
    let zsh = CString::new("/bin/zsh").unwrap();
    nix::unistd::execv::<&CStr>(&zsh, &[]).whatever("error executing shell")
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateInfo {
    pub description: String,
}
