//! The `cache` command.

use std::path::Path;

use crate::cli::{args, load_project};
use crate::BakeryResult;

/// Run the `list` command.
pub fn run(args: &args::Args, cmd: &args::CacheCommand) -> BakeryResult<()> {
    let project = load_project(args)?;
    match cmd {
        args::CacheCommand::Clean => {
            std::fs::remove_dir_all("/project/.rugix").ok();
            std::fs::remove_dir_all(
                Path::new("/run/rugix/bakery/cache").join(project.local_id().as_str()),
            )
            .ok();
        }
    }
    Ok(())
}
