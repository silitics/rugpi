//! The `test` command.

use std::ffi::OsStr;
use std::fs;

use reportify::ResultExt;

use crate::cli::{args, load_project};
use crate::{tester, BakeryResult};

/// Run the `test` command.
pub fn run(args: &args::Args, cmd: &args::TestCommand) -> BakeryResult<()> {
    let project = load_project(args)?;
    let mut workflows = Vec::new();
    if cmd.workflows.is_empty() {
        let mut read_dir = fs::read_dir(project.dir().join("tests"))
            .whatever("unable to scan for test workflows")?;
        while let Some(entry) = read_dir.next() {
            let entry = entry.whatever("unable to read entry")?;
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
        tester::main(&project, &workflow)?;
        rugix_cli::force_redraw();
    }
    Ok(())
}
