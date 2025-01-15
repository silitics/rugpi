//! Implementation of the CLI.

use std::path::PathBuf;

use reportify::ResultExt;

use crate::project::{ProjectLoader, ProjectRef};
use crate::BakeryResult;

mod cmds;

pub mod args;

pub(crate) mod status;

/// Run Rugix Bakery with the provided command line arguments.
pub async fn run(args: args::Args) -> BakeryResult<()> {
    match &args.cmd {
        args::Command::Bake(cmd) => cmds::run_bake::run(&args, cmd).await,
        args::Command::Test(cmd) => cmds::run_test::run(&args, cmd).await,
        args::Command::List(cmd) => cmds::run_list::run(&args, cmd).await,
        args::Command::Pull => cmds::run_pull::run(&args).await,
        args::Command::Init(cmd) => cmds::run_init::run(cmd).await,
        args::Command::Shell => cmds::run_shell::run().await,
    }
}

/// Get the current working directory.
fn current_dir() -> BakeryResult<PathBuf> {
    std::env::current_dir().whatever("unable to get current working directory")
}

/// Load the project from the current working directory.
async fn load_project(args: &args::Args) -> BakeryResult<ProjectRef> {
    ProjectLoader::current_dir()?
        .with_config_file(args.config.as_deref())
        .load()
        .await
}
