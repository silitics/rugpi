//! The `test` command.

use std::ffi::OsStr;

use reportify::ResultExt;

use crate::cli::{args, load_project};
use crate::{tester, BakeryResult};

/// Run the `test` command.
pub async fn run(args: &args::Args, cmd: &args::TestCommand) -> BakeryResult<()> {
    let project = load_project(args).await?;
    let mut workflows = Vec::new();
    if cmd.workflows.is_empty() {
        let mut read_dir = tokio::fs::read_dir(project.dir().join("tests"))
            .await
            .whatever("unable to scan for test workflows")?;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .whatever("unable to read entry")?
        {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("toml")) {
                workflows.push(path);
            }
        }
    } else {
        for name in &cmd.workflows {
            workflows.push(
                project
                    .dir()
                    .join("tests")
                    .join(name)
                    .with_extension("toml"),
            );
        }
    };
    for workflow in &workflows {
        tester::main(&project, &workflow).await?;
        rugpi_cli::force_redraw();
    }
    Ok(())
}
