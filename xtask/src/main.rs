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
    Doc,
    BuildImage,
}

pub fn project_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let env = LocalEnv::new(project_path());
    match args.task {
        Task::BuildImage => {
            run!(
                env,
                [
                    "docker",
                    "build",
                    "-t",
                    "ghcr.io/silitics/rugpi-bakery:dev",
                    "-f",
                    "docker/Dockerfile.rugpi-bakery",
                    "."
                ]
                .with_stdout(Out::Inherit)
                .with_stderr(Out::Inherit)
            )?;
        }
        Task::Doc => {
            run!(
                env,
                ["cargo", "+nightly", "doc", "--document-private-items",]
                    .with_stdout(Out::Inherit)
                    .with_stderr(Out::Inherit)
            )?;
        }
    }
    Ok(())
}
