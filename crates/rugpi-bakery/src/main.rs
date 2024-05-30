use std::{
    convert::Infallible,
    ffi::{CStr, CString},
    fs,
    path::{Path, PathBuf},
};

use anyhow::bail;
use bake::{image::make_image, LayerBakery};
use clap::Parser;
use colored::Colorize;
use project::{
    config::Architecture, images::ImageConfig, repositories::Source, Project, ProjectLoader,
};
use rugpi_common::{fsutils::copy_recursive, Anyhow};
use utils::logging::init_logging;

pub mod bake;
pub mod project;
pub mod utils;

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
fn main() -> Anyhow<()> {
    init_logging();

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
            std::fs::write("run-bakery", interpolate_run_bakery(version))?;
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
                let config: ImageConfig = toml::from_str(&fs::read_to_string(&config)?)?;
                make_image(&config, source, image)?;
            }
        },
        Command::Init(cmd) => {
            let Some(template) = &cmd.template else {
                for entry in std::fs::read_dir("/usr/share/rugpi/templates")? {
                    let entry = entry?;
                    if !entry.file_type()?.is_dir() {
                        continue;
                    }
                    let file_name = entry.file_name();
                    let template_name = file_name.to_str().expect("template names should be UTF-8");
                    println!("{template_name}");
                }
                return Ok(());
            };
            if Path::new("rugpi-bakery.toml").exists() {
                bail!("Project has already been initialized.");
            }
            let template_dir = Path::new("/usr/share/rugpi/templates").join(template);
            copy_recursive(template_dir, "/project")?;
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
    }
    Ok(())
}

fn interpolate_run_bakery(version: &str) -> String {
    include_str!("../assets/run-bakery").replace("%%DEFAULT_VERSION%%", version)
}

fn load_project(args: &Args) -> Anyhow<Project> {
    ProjectLoader::current_dir()?
        .with_config_file(args.config.as_deref())
        .load()
}

fn exec_shell() -> Anyhow<Infallible> {
    let zsh = CString::new("/bin/zsh").unwrap();
    Ok(nix::unistd::execv::<&CStr>(&zsh, &[])?)
}
