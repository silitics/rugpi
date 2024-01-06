use std::{
    ffi::{CStr, CString},
    path::PathBuf,
};

use clap::Parser;
use colored::Colorize;
use project::{
    repositories::{PathSource, Repositories, Source},
    ProjectLoader,
};
use rugpi_common::Anyhow;
use tasks::{
    bake::{self, BakeTask},
    customize::{self, CustomizeTask},
    extract::ExtractTask,
};

pub mod project;
pub mod tasks;
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
pub enum Task {
    /// Extract all system files from a given base image.
    Extract(ExtractTask),
    /// Apply modification to the system.
    Customize(CustomizeTask),
    /// Bake a final image for distribution.
    Bake(BakeTask),
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
        Task::Extract(task) => {
            task.run()?;
        }
        Task::Customize(task) => {
            customize::run(&project, task)?;
        }
        Task::Bake(task) => {
            bake::run(&project, task)?;
        }
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
            let mut repositories = Repositories::new(&project.dir);
            repositories.load_source(
                Source::Path(PathSource {
                    path: "/usr/share/rugpi/repositories/core".into(),
                }),
                false,
            )?;
            repositories.load_root(project.config.repositories.clone(), true)?;
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
