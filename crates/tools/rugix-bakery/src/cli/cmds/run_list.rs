//! The `list` command.

use crate::cli::{args, load_project};
use crate::BakeryResult;

/// Run the `list` command.
pub fn run(args: &args::Args, cmd: &args::ListCommand) -> BakeryResult<()> {
    let project = load_project(args)?;
    match cmd {
        args::ListCommand::Systems => {
            rugix_cli::suspend(|| {
                if let Some(systems) = &project.config().systems {
                    eprintln!("Available Systems:");
                    for name in systems.keys() {
                        eprintln!("  {name}");
                    }
                } else {
                    eprintln!("No systems available.");
                }
            });
        }
    }
    Ok(())
}
