use std::path::PathBuf;

use clap::Parser;
use xscript::{run, LocalEnv, Out, Run};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    task: Task,
}

#[derive(Debug, Parser)]
pub enum Task {
    BuildImage,
}

pub fn project_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.task {
        Task::BuildImage => {
            let env = LocalEnv::new(project_path());
            run!(
                env,
                [
                    "docker",
                    "build",
                    "-t",
                    "ghcr.io/silitics/rugpi-bakery:latest",
                    "-f",
                    "docker/Dockerfile.rugpi-bakery",
                    "."
                ]
                .with_stdout(Out::Inherit)
                .with_stderr(Out::Inherit)
            )?;
        }
    }
    Ok(())
}
