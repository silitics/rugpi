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
    BuildBinaries { target: Option<String> },
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
                    "bakery/Dockerfile",
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
        Task::BuildBinaries { target } => {
            let target = target.as_deref().unwrap_or("aarch64-unknown-linux-musl");
            run!(
                env,
                ["cargo", "build", "--release", "--target", target]
                    .with_stdout(Out::Inherit)
                    .with_stderr(Out::Inherit)
            )?;
            let binaries_dir = project_path().join("build/binaries").join(target);
            if binaries_dir.exists() {
                std::fs::remove_dir_all(&binaries_dir)?;
            }
            std::fs::create_dir_all(&binaries_dir)?;
            let target_dir = project_path().join("target").join(target).join("release");
            for entry in std::fs::read_dir(&target_dir)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                if !file_type.is_file() {
                    continue;
                }
                let Ok(file_name) = entry.file_name().into_string() else {
                    continue;
                };
                if !file_name.starts_with("rugpi-") || file_name.ends_with(".d") {
                    continue;
                }
                std::fs::hard_link(entry.path(), binaries_dir.join(file_name))?;
            }
        }
    }
    Ok(())
}
