//! The `bake` command.

use std::path::Path;

use reportify::ResultExt;

use crate::cli::{args, load_project};
use crate::oven::LayerBakery;
use crate::{oven, BakeryResult};

/// Run the `bake` command.
pub fn run(args: &args::Args, cmd: &args::BakeCommand) -> BakeryResult<()> {
    let project = load_project(args)?;
    match cmd {
        args::BakeCommand::Image { system, output } => {
            let image_path = output
                .clone()
                .unwrap_or_else(|| Path::new("build/images").join(system).with_extension("img"));
            let system_path = Path::new(".rugpi/systems").join(system);
            oven::bake_system(&project, system, &system_path)?;
            if let Some(parent) = image_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::copy(system_path.join("system.img"), image_path)
                .whatever("error copying image")?;
        }
        args::BakeCommand::Layer { layer, arch } => {
            LayerBakery::new(&project, *arch).bake_root(layer)?;
        }
        args::BakeCommand::Bundle {
            system,
            output,
            opts,
        } => {
            let system_path = Path::new(".rugpi/systems").join(system);
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
