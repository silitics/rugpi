use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use rugix_tasks::block_on;
use tracing::info;

use reportify::{ErrorExt, ResultExt};

use rugpi_cli::style::{Style, Stylize};
use rugpi_cli::widgets::{Heading, ProgressBar, ProgressSpinner, Text, Widget};
use rugpi_cli::{StatusSegment, StatusSegmentRef, VisualHeight};

use crate::config::load_config;
use crate::config::tests::{RunStep, TestConfig, WaitStep};
use crate::project::ProjectRef;
use crate::{oven, BakeryResult};

pub mod qemu;

pub fn main(project: &ProjectRef, test_path: &Path) -> BakeryResult<()> {
    let test_config = load_config::<TestConfig>(test_path)?;

    let test_name = test_path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    for system in &test_config.systems {
        let output = Path::new("build/images")
            .join(&system.disk_image)
            .with_extension("img");
        {
            let output = output.clone();
            let project = project.clone();
            let disk_image = system.disk_image.clone();
            oven::bake_image(&project, &disk_image, &output).whatever("error baking image")?;
        }

        let test_status = rugpi_cli::add_status(TestCliStatus {
            total_steps: test_config.steps.len() as u64,
            state: Mutex::default(),
            heading: format!("Test {test_name:?}"),
        });

        let image_config = project.config().resolve_image_config(&system.disk_image)?;

        block_on(async {
            let vm = qemu::start(
                image_config.architecture,
                &output.to_string_lossy(),
                &system,
            )
            .await?;

            info!("VM started");

            let ctx = TestCtx {
                status: test_status.clone(),
            };

            for (idx, step) in test_config.steps.iter().enumerate() {
                test_status.state.lock().unwrap().current_step = idx as u64 + 1;
                rugpi_cli::redraw();
                match step {
                    crate::config::tests::TestStep::Run(RunStep {
                        description,
                        script,
                        stdin_file,
                        may_disconnect,
                        may_fail,
                    }) => {
                        info!("running script");
                        ctx.status
                            .set_description(description.clone().unwrap_or_default());
                        {
                            let mut state = ctx.status.state.lock().unwrap();
                            state.step_progress = Some(StepProgress {
                                message: "waiting for SSH to connect",
                                position: 0,
                                length: None,
                            });
                        }
                        vm.wait_for_ssh()
                            .await
                            .whatever("unable to connect to VM via SSH")?;
                        {
                            let mut state = ctx.status.state.lock().unwrap();
                            state.step_progress = None;
                        }
                        if let Err(report) = vm
                            .run_script(&ctx, script, stdin_file.as_ref().map(|p| p.as_ref()))
                            .await
                        {
                            match report.error() {
                                qemu::ExecError::Disconnected => {
                                    if !may_disconnect.unwrap_or(false) {
                                        return Err(report.whatever("script execution failed"));
                                    }
                                }
                                qemu::ExecError::Failed { code } => {
                                    if *code != 0 && !may_fail.unwrap_or(false) {
                                        return Err(report.whatever("script execution failed"));
                                    }
                                }
                                qemu::ExecError::Other => {
                                    return Err(report.whatever("script execution failed"));
                                }
                            }
                        }
                    }
                    crate::config::tests::TestStep::Wait(WaitStep {
                        description,
                        duration,
                    }) => {
                        ctx.status
                            .set_description(description.clone().unwrap_or_else(|| {
                                if *duration == 1.0 {
                                    "wait for 1 second".to_owned()
                                } else {
                                    format!("wait for {duration:.1} seconds")
                                }
                            }));
                        tokio::time::sleep(Duration::from_secs_f64(*duration)).await;
                    }
                }
            }

            Ok(())
        })?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct TestCtx {
    pub status: StatusSegmentRef<TestCliStatus>,
}

#[derive(Debug)]
pub struct TestCliStatus {
    state: Mutex<TestState>,
    heading: String,
    total_steps: u64,
}

impl TestCliStatus {
    pub fn set_description(&self, description: String) {
        self.state.lock().unwrap().description = description;
    }

    pub fn push_log_line(&self, line: String) {
        let mut state = self.state.lock().unwrap();
        state.log_lines.push_back(line);
        while state.log_lines.len() > 10 {
            state.log_lines.pop_front();
        }
    }
}

#[derive(Debug, Default)]
struct TestState {
    current_step: u64,
    log_lines: VecDeque<String>,
    description: String,
    step_progress: Option<StepProgress>,
}

impl StatusSegment for TestCliStatus {
    fn draw(&self, ctx: &mut rugpi_cli::DrawCtx) {
        let state = self.state.lock().unwrap();
        Heading::new(&self.heading).draw(ctx);
        ProgressSpinner::new().draw(ctx);
        ctx.with_style(Style::new().bold(), |ctx| {
            write!(ctx, " Step {}/{}:", state.current_step, self.total_steps);
        });
        if !state.description.is_empty() {
            write!(ctx, " {:?}", state.description);
        }
        if let Some(step_progress) = &state.step_progress {
            write!(ctx, "\n╰╴{} ", step_progress.message);
            if let Some(length) = step_progress.length {
                ProgressBar::new(step_progress.position, length)
                    .hide_percentage()
                    .draw(ctx);
            } else {
                ctx.start_line();
            }
        }
        if !state.log_lines.is_empty() {
            let show_lines = VisualHeight::from_usize(state.log_lines.len())
                .min(ctx.measure_remaining_height())
                .into_u64() as usize;
            let skip_lines = state.log_lines.len() - show_lines;
            Text::new(state.log_lines.iter().skip(skip_lines))
                .prefix("> ")
                .styled()
                .dark_gray()
                .draw(ctx);
        }
    }
}

#[derive(Debug)]
pub struct StepProgress {
    message: &'static str,
    position: u64,
    length: Option<u64>,
}
