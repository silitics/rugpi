use std::path::PathBuf;

use clap::Parser;
use rugpi_common::Anyhow;
use tasks::{
    bake::{self, BakeTask},
    customize::{self, CustomizeTask},
    extract::ExtractTask,
};

pub mod config;
pub mod recipes;
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
}

fn main() -> Anyhow<()> {
    let args = Args::parse();
    match &args.task {
        Task::Extract(task) => {
            task.run()?;
        }
        Task::Customize(task) => {
            customize::run(&args, task)?;
        }
        Task::Bake(task) => {
            bake::run(&args, task)?;
        }
    }
    Ok(())
}
