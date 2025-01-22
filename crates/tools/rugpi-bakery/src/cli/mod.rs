//! Implementation of the CLI.

use std::path::PathBuf;

use reportify::ResultExt;

use crate::project::{ProjectLoader, ProjectRef};
use crate::BakeryResult;

mod cmds;

pub mod args;

pub(crate) mod status;

/// Run Rugix Bakery with the provided command line arguments.
pub fn run(args: args::Args) -> BakeryResult<()> {
    match &args.cmd {
        args::Command::Bake(cmd) => cmds::run_bake::run(&args, cmd),
        args::Command::Test(cmd) => cmds::run_test::run(&args, cmd),
        args::Command::Run(cmd) => cmds::run_run::run(&args, cmd),
        args::Command::List(cmd) => cmds::run_list::run(&args, cmd),
        args::Command::Pull => cmds::run_pull::run(&args),
        args::Command::Init(cmd) => cmds::run_init::run(cmd),
        args::Command::Shell => cmds::run_shell::run(),
    }
}

/// Get the current working directory.
fn current_dir() -> BakeryResult<PathBuf> {
    std::env::current_dir().whatever("unable to get current working directory")
}

/// Load the project from the current working directory.
fn load_project(args: &args::Args) -> BakeryResult<ProjectRef> {
    ProjectLoader::current_dir()?
        .with_config_file(args.config.as_deref())
        .load()
}
