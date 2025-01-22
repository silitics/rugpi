//! Definition of the command line arguments.

use std::path::PathBuf;

use clap::Parser;

use crate::config::projects::Architecture;

/// Command line arguments.
#[derive(Debug, Parser)]
#[command(author, about = None, long_about = None)]
pub struct Args {
    /// Path to the `rugix-bakery.toml` configuration file.
    #[clap(long)]
    pub config: Option<PathBuf>,
    /// The command to execute.
    #[clap(subcommand)]
    pub cmd: Command,
}

/// Commands of the CLI.
#[derive(Debug, Parser)]
pub enum Command {
    /// Bake an image or a layer.
    #[clap(subcommand)]
    Bake(BakeCommand),
    /// Run integration tests.
    Test(TestCommand),
    /// Run an image in a VM.
    Run(RunCommand),
    /// List images, recipes, and layers.
    #[clap(subcommand)]
    List(ListCommand),
    /// Pull in external repositories.
    Pull,
    /// Initialize the project from a template.
    Init(InitCommand),
    /// Spawn a shell in the Rugpi Bakery Docker container.
    Shell,
}

/// The `list` command.
#[derive(Debug, Parser)]
pub enum ListCommand {
    /// List available images.
    Images,
}

/// The `bake` command.
#[derive(Debug, Parser)]
pub enum BakeCommand {
    /// Bake an image.
    Image {
        /// The name of the image to bake.
        image: String,
        /// The output path of the resulting image.
        output: Option<PathBuf>,
    },
    /// Bake a layer.
    Layer {
        /// The architecture to bake the layer for.
        #[clap(long)]
        arch: Architecture,
        /// The name of the layer to bake.
        layer: String,
    },
}

/// The `test` command.
#[derive(Debug, Parser)]
pub struct TestCommand {
    pub workflows: Vec<String>,
}

/// The `run` command.
#[derive(Debug, Parser)]
pub struct RunCommand {
    pub image: String,
}

/// The `bake` command.
#[derive(Debug, Parser)]
pub enum InternalCommand {
    MakeImage {
        config: PathBuf,
        source: PathBuf,
        image: PathBuf,
    },
}

/// The `init` command.
#[derive(Debug, Parser)]
pub struct InitCommand {
    /// Template to use.
    pub template: Option<String>,
}
