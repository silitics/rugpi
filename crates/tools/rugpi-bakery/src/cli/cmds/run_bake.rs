//! The `bake` command.

use std::path::Path;

use crate::cli::{args, load_project};
use crate::oven::LayerBakery;
use crate::{oven, BakeryResult};

/// Run the `bake` command.
pub fn run(args: &args::Args, cmd: &args::BakeCommand) -> BakeryResult<()> {
    let project = load_project(args)?;
    match cmd {
        args::BakeCommand::System { system, output } => {
            let output = output
                .clone()
                .unwrap_or_else(|| Path::new("build/systems").join(system));
            oven::bake_system(&project, system, &output)?;
        }
        args::BakeCommand::Layer { layer, arch } => {
            LayerBakery::new(&project, *arch).bake_root(layer)?;
        }
        args::BakeCommand::Bundle {
            system,
            output,
            opts,
        } => {
            let system_path = Path::new("build/systems").join(system);
            oven::bake_system(&project, system, &system_path)?;
            let output = output.clone().unwrap_or_else(|| {
                Path::new("build/bundles")
                    .join(system)
                    .with_extension("rugixb")
            });
            oven::bake_bundle(&project, system, &system_path, &output, opts)?;
        }
    }
    Ok(())
}
