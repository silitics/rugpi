//! The `list` command.

use crate::cli::{args, load_project};
use crate::BakeryResult;

/// Run the `list` command.
pub fn run(args: &args::Args, cmd: &args::ListCommand) -> BakeryResult<()> {
    let project = load_project(args)?;
    match cmd {
        args::ListCommand::Images => {
            rugpi_cli::suspend(|| {
                if let Some(images) = &project.config().images {
                    eprintln!("Available Images:");
                    for name in images.keys() {
                        eprintln!("  {name}");
                    }
                } else {
                    eprintln!("No images available.");
                }
            });
        }
    }
    Ok(())
}
