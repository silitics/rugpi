use std::convert::Infallible;
use std::ffi::{CStr, OsStr};
use std::path::Path;
use std::thread;
use std::time::Duration;

use reportify::{Report, ResultExt};
use rugpi_common::DropGuard;
use serde::Deserialize;

use crate::bake::LayerBakery;
use crate::project::{Project, ProjectLoader};
use crate::test::{self, RugpiTestError};
use crate::{cli, BakeryResult};

pub async fn run(args: cli::Args) -> BakeryResult<()> {
    // match &args.command {
    //     cli::Command::Bake(command) => {
    //         let project = load_project(&args)?;
    //         match command {
    //             cli::BakeCommand::Image { image, output } => {
    //                 let output = output.clone().unwrap_or_else(|| {
    //                     Path::new("build/images").join(image).with_extension("img")
    //                 });
    //                 bake::bake_image(&project, image, &output)?;
    //             }
    //             cli::BakeCommand::Layer { layer, arch } => {
    //                 LayerBakery::new(&project, *arch).bake_root(layer)?;
    //             }
    //         }
    //     }
    //     cli::Command::Shell => {
    //         exec_shell()?;
    //     }
    //     cli::Command::Update(task) => {
    //         let version = task.version.as_deref().unwrap_or("v0");
    //         println!("Switch Rugpi Bakery to version `{version}`...");
    //         std::fs::write("run-bakery", interpolate_run_bakery(version))
    //             .whatever("error writing `run-bakery`")?;
    //     }
    //     cli::Command::Pull => {
    //         let project = load_project(&args)?;
    //         for (_, repository) in project.repositories()?.iter() {
    //             println!(
    //                 "{} {} {}",
    //                 repository.source.id.as_short_str().blue(),
    //                 repository.config.name.as_deref().unwrap_or("<unknown>"),
    //                 repository
    //                     .config
    //                     .description
    //                     .as_deref()
    //                     .unwrap_or("")
    //                     .bright_black(),
    //             );
    //             match &repository.source.source {
    //                 Source::Path(path_source) => {
    //                     println!(
    //                         "  {}{}",
    //                         "source path ./".bright_black(),
    //                         path_source.path.to_string_lossy().bright_black()
    //                     );
    //                 }
    //                 Source::Git(git_source) => {
    //                     println!(
    //                         "  {}{}",
    //                         "source git ".bright_black(),
    //                         git_source.url.bright_black()
    //                     );
    //                 }
    //             }
    //         }
    //     }
    //     cli::Command::Internal(cmd) => match cmd {
    //         cli::InternalCommand::MakeImage {
    //             config,
    //             source,
    //             image,
    //         } => {
    //             let config: ImageConfig = toml::from_str(
    //                 &fs::read_to_string(&config).whatever("error reading image
    // config")?,             )
    //             .whatever("error parsing image config")?;
    //             make_image(&config, source, image)?;
    //         }
    //     },
    //     cli::Command::Init(cmd) => {
    //         let Some(template) = &cmd.template else {
    //             let templates: HashMap<String, TemplateInfo> = toml::from_str(
    //
    // &std::fs::read_to_string("/usr/share/rugpi/templates/templates.toml")
    //                     .whatever("error reading templates list")?,
    //             )
    //             .whatever("error parsing templates list")?;
    //             println!("{}\n", "Available Templates:".bold());
    //             let mut names = templates.keys().collect::<Vec<_>>();
    //             names.sort();
    //             for name in names {
    //                 let info = templates.get(name).unwrap();
    //                 println!(
    //                     "  {}:\n    {}",
    //                     name.blue(),
    //                     info.description.trim().bright_black()
    //                 );
    //             }
    //             return Ok(());
    //         };
    //         if Path::new("rugpi-bakery.toml").exists() {
    //             bail!("Project has already been initialized.");
    //         }
    //         let template_dir = Path::new("/usr/share/rugpi/templates").join(template);
    //         copy_recursive(template_dir, "/project")
    //             .whatever("error copying template to project directory")?;
    //     }
    //     cli::Command::List(cmd) => {
    //         let project = load_project(&args)?;
    //         match cmd {
    //             cli::ListCommand::Images => {
    //                 println!("Available Images:");
    //                 for name in project.config.images.keys() {
    //                     println!("  {name}");
    //                 }
    //             }
    //         }
    //     }
    //     cli::Command::Test(test_command) => {
    //         let project = load_project(&args)?;
    //         tokio::runtime::Builder::new_multi_thread()
    //             .enable_all()
    //             .build()
    //             .unwrap()
    //             .block_on(async move {
    //                 let mut workflows = Vec::new();
    //                 if test_command.workflows.is_empty() {
    //                     let mut read_dir =
    // tokio::fs::read_dir(project.dir.join("tests"))                         .await
    //                         .whatever("unable to scan for test workflows")?;
    //                     while let Some(entry) = read_dir
    //                         .next_entry()
    //                         .await
    //                         .whatever("unable to read entry")?
    //                     {
    //                         let path = entry.path();
    //                         if path.extension() == Some(OsStr::new("toml")) {
    //                             workflows.push(path);
    //                         }
    //                     }
    //                 } else {
    //                     for name in &test_command.workflows {
    //                         workflows
    //
    // .push(project.dir.join("tests").join(name).with_extension("toml"));
    // }                 };
    //                 for workflow in workflows {
    //                     test::main(&project, &workflow).await?;
    //                     rugpi_cli::force_redraw();
    //                 }
    //                 <Result<(), Report<RugpiTestError>>>::Ok(())
    //             })
    //             .whatever("unable to run test")?;
    //     }
    // }
    // Ok(())
}

fn interpolate_run_bakery(version: &str) -> String {
    include_str!("../assets/run-bakery").replace("%%DEFAULT_VERSION%%", version)
}

fn load_project(args: &cli::Args) -> BakeryResult<Project> {
    ProjectLoader::current_dir()?
        .with_config_file(args.config.as_deref())
        .load()
}

fn exec_shell() -> BakeryResult<Infallible> {
    nix::unistd::execv::<&CStr>(c"/bin/zsh", &[]).whatever("error executing shell")
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateInfo {
    pub description: String,
}
