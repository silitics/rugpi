use clap::Parser;
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
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Parser)]
pub enum Cmd {
    /// Extract all system files from a given base image.
    Extract(ExtractTask),
    /// Apply modification to the system.
    Customize(CustomizeTask),
    /// Bake a final image for distribution.
    Bake(BakeTask),
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match &args.cmd {
        Cmd::Extract(task) => {
            task.run()?;
        }
        Cmd::Customize(args) => {
            customize::run(args)?;
        }
        Cmd::Bake(task) => {
            bake::run(task)?;
        }
    }
    Ok(())
}
