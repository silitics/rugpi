use std::{
    convert::Infallible,
    ffi::{CStr, CString},
    fs,
    path::PathBuf,
};

use bake::{image::make_image, LayerBakery};
use clap::Parser;
use colored::Colorize;
use project::{
    config::Architecture, images::ImageConfig, repositories::Source, Project, ProjectLoader,
};
use rugpi_common::Anyhow;
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
    /// Spawn a shell in the Rugpi Bakery Docker container.
    Shell,
    /// Pull in external repositories.
    Pull,
    /// Update Rugpi Bakery itself.
    Update(UpdateCommand),
    /// Internal unstable commands.
    #[clap(subcommand)]
    Internal(InternalCommand),
}

/// The `bake` command.
#[derive(Debug, Parser)]
pub enum BakeCommand {
    /// Bake an image.
    Image {
        /// The name of the image to bake.
        image: String,
        /// The output path of the resulting image.
        output: PathBuf,
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

/// Entrypoint of the CLI.
fn main() -> Anyhow<()> {
    init_logging();

    let args = Args::parse();
    let project = load_project(&args)?;
    match &args.command {
        Command::Bake(command) => match command {
            BakeCommand::Image { image, output } => {
                bake::bake_image(&project, image, output)?;
            }
            BakeCommand::Layer { layer, arch } => {
                LayerBakery::new(&project, *arch).bake_root(layer)?;
            }
        },
        Command::Shell => {
            exec_shell()?;
        }
        Command::Update(task) => {
            let version = task.version.as_deref().unwrap_or("v0");
            println!("Switch Rugpi Bakery to version `{version}`...");
            std::fs::write("run-bakery", interpolate_run_bakery(version))?;
        }
        Command::Pull => {
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
