//! The `init` command.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use colored::Colorize;
use serde::Deserialize;

use reportify::{bail, ResultExt};

use rugix_common::fsutils::copy_recursive;

use crate::cli::{args, current_dir};
use crate::BakeryResult;

/// Run the `init` command.
pub fn run(cmd: &args::InitCommand) -> BakeryResult<()> {
    let Some(template_name) = &cmd.template else {
        let templates: HashMap<String, TemplateInfo> = toml::from_str(
            &fs::read_to_string(Path::new(TEMPLATE_PATH).join("templates.toml"))
                .whatever("error reading templates list")?,
        )
        .whatever("error parsing templates list")?;

        rugix_cli::suspend(|| {
            eprintln!("{}\n", "Available Templates:".bold());
            let mut names = templates.keys().collect::<Vec<_>>();
            names.sort();
            for name in names {
                let info = templates.get(name).unwrap();
                eprintln!(
                    "  {}:\n    {}",
                    name.blue(),
                    info.description.trim().bright_black()
                );
            }
        });
        return Ok(());
    };
    if Path::new("rugix-bakery.toml").exists() {
        bail!("Project has already been initialized.");
    }
    let cwd = current_dir()?;
    let template_dir = Path::new(TEMPLATE_PATH).join(template_name);
    copy_recursive(template_dir, cwd).whatever("error copying template to project directory")?;
    Ok(())
}

/// Template path.
const TEMPLATE_PATH: &str = "/usr/share/rugix/templates";

/// Template information.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateInfo {
    /// Description of the template.
    pub description: String,
}
