use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use reportify::{bail, Report, ResultExt};
use rugpi_cli::widgets::{Heading, ProgressBar, Widget};
use rugpi_cli::StatusSegment;
use tokio::fs;
use tokio::task::spawn_blocking;
use tracing::info;
use workflow::{TestStep, TestWorkflow};

use crate::bake;
use crate::project::Project;

pub mod qemu;
pub mod workflow;

reportify::new_whatever_type! {
    RugpiTestError
}

pub type RugpiTestResult<T> = Result<T, Report<RugpiTestError>>;

pub async fn main(project: &Project, workflow_path: &Path) -> RugpiTestResult<()> {
    let workflow = toml::from_str::<TestWorkflow>(
        &fs::read_to_string(&workflow_path)
            .await
            .whatever("unable to read test workflow")?,
    )
    .whatever("unable to parse test workflow")?;

    let test_name = workflow_path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    for system in &workflow.systems {
        let output = Path::new("build/images")
            .join(&system.disk_image)
            .with_extension("img");
        let project = project.clone();
        let disk_image = system.disk_image.clone();
        {
            let output = output.clone();
            spawn_blocking(move || bake::bake_image(&project, &disk_image, &output))
                .await
                .whatever("error baking image")?
                .whatever("error baking image")?;
        }

        let test_status = rugpi_cli::add_status(TestCliStatus {
            total_steps: workflow.steps.len() as u64,
            state: Mutex::new(TestState { current_step: 0 }),
            heading: format!("Test {test_name:?}"),
        });

        let vm = qemu::start(&output.to_string_lossy(), system).await?;

        info!("VM started");

        for (idx, step) in workflow.steps.iter().enumerate() {
            test_status.state.lock().unwrap().current_step = idx as u64 + 1;
            rugpi_cli::redraw();
            match step {
                workflow::TestStep::Run {
                    script,
                    stdin,
                    may_fail,
                } => {
                    info!("running script");
                    vm.wait_for_ssh()
                        .await
                        .whatever("unable to connect to VM via SSH")?;
                    if let Err(report) = vm
                        .run_script(script, stdin.as_ref().map(|p| p.as_ref()))
                        .await
                        .whatever::<RugpiTestError, _>("unable to run script")
                    {
                        if may_fail.unwrap_or(false) {
                            eprintln!("ignoring error while executing script:\n{report:?}");
                        } else {
                            bail!("error during test")
                        }
                    }
                }
                TestStep::Wait { duration } => {
                    info!("waiting for {duration} seconds");
                    tokio::time::sleep(Duration::from_secs_f64(*duration)).await;
                }
            }
        }
    }

    Ok(())
}

pub struct TestCliStatus {
    state: Mutex<TestState>,
    heading: String,
    total_steps: u64,
}

struct TestState {
    current_step: u64,
}

impl StatusSegment for TestCliStatus {
    fn draw(&self, ctx: &mut rugpi_cli::DrawCtx) {
        let state = self.state.lock().unwrap();
        Heading::new(&self.heading).draw(ctx);
        write!(ctx, "Step [{}/{}] ", state.current_step, self.total_steps);
        ProgressBar::new(state.current_step - 1, self.total_steps).draw(ctx)
    }
}
