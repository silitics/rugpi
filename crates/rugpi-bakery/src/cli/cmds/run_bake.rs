//! The `bake` command.

use std::path::Path;

use crate::cli::{args, load_project};
use crate::oven::LayerBakery;
use crate::{oven, BakeryResult};

/// Run the `bake` command.
pub async fn run(args: &args::Args, cmd: &args::BakeCommand) -> BakeryResult<()> {
    let project = load_project(args).await?;
    match cmd {
        args::BakeCommand::Image { image, output } => {
            let output = output
                .clone()
                .unwrap_or_else(|| Path::new("build/images").join(image).with_extension("img"));
            oven::bake_image(&project, image, &output).await?;
        }
        args::BakeCommand::Layer { layer, arch } => {
            LayerBakery::new(&project, *arch).bake_root(layer).await?;
        }
    }
    Ok(())
}
