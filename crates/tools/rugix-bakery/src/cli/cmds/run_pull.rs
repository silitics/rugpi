//! The `pull` command.

use colored::Colorize;

use crate::cli::{args, load_project};
use crate::config::repositories::SourceConfig;
use crate::BakeryResult;

/// Run the `pull` command.
pub fn run(args: &args::Args) -> BakeryResult<()> {
    let project = load_project(args)?;
    for (_, repository) in project.repositories()?.iter() {
        rugix_cli::suspend(|| {
            println!(
                "{} {} {}",
                repository.source.id.as_short_str().blue(),
                repository.config.name.as_deref().unwrap_or("<unknown>"),
                repository
                    .config
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .bright_black(),
            );
            match &repository.source.config {
                SourceConfig::Path(config) => {
                    println!(
                        "  {}{}",
                        "source path ./".bright_black(),
                        config.path.bright_black()
                    );
                }
                SourceConfig::Git(config) => {
                    println!(
                        "  {}{}",
                        "source git ".bright_black(),
                        config.url.bright_black()
                    );
                }
            }
        });
    }
    Ok(())
}
