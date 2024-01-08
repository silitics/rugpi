use std::{
    ffi::{CStr, CString},
    path::PathBuf,
};

use clap::Parser;
use colored::Colorize;
use project::{config::Architecture, repositories::Source, ProjectLoader};
use rugpi_common::Anyhow;

pub mod bake;
pub mod idx_vec;
pub mod project;
pub mod utils;

#[derive(Debug, Parser)]
pub struct Args {
    /// Path to `rugpi-bakery.toml` configuration file.
    #[clap(long)]
    config: Option<PathBuf>,
    /// The task to execute.
    #[clap(subcommand)]
    task: Task,
}

#[derive(Debug, Parser)]
pub enum BakeCommand {
    Image {
        image: String,
        output: PathBuf,
    },
    Layer {
        #[clap(long)]
        arch: Architecture,
        layer: String,
    },
}

#[derive(Debug, Parser)]
pub enum Task {
    // /// Extract all system files from a given base image.
    // Extract(ExtractTask),
    // /// Apply modification to the system.
    // Customize(CustomizeTask),
    /// Bake a final image for distribution.
    #[clap(subcommand)]
    Bake(BakeCommand),
    /// Spawn a shell in the Rugpi Bakery Docker container.
    Shell,
    Update(UpdateTask),
    /// Pull in external repositories.
    Pull,
}

#[derive(Debug, Parser)]
pub struct UpdateTask {
    version: Option<String>,
}

fn main() -> Anyhow<()> {
    let args = Args::parse();
    let project = ProjectLoader::current_dir()?
        .with_config_file(args.config.as_deref())
        .load()?;
    match &args.task {
        Task::Bake(task) => match task {
            BakeCommand::Image { image, output } => {
                bake::bake_image(&project, image, output)?;
            }
            BakeCommand::Layer { layer, arch } => {
                bake::bake_layer(&project, *arch, layer)?;
            }
        },
        Task::Shell => {
            let zsh_prog = CString::new("/bin/zsh")?;
            nix::unistd::execv::<&CStr>(&zsh_prog, &[])?;
        }
        Task::Update(task) => {
            let version = task.version.as_deref().unwrap_or("v0");
            println!("Switch Rugpi Bakery to version `{version}`...");
            std::fs::write("run-bakery", interpolate_run_bakery(version))?;
        }
        Task::Pull => {
            let repositories = project.load_repositories()?;
            for (_, repository) in repositories.iter() {
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
    }
    Ok(())
}

fn interpolate_run_bakery(version: &str) -> String {
    include_str!("../assets/run-bakery").replace("%%DEFAULT_VERSION%%", version)
}
