use std::{path::PathBuf, time::Duration};

use case::{TestCase, TestStep};
use clap::Parser;
use reportify::{bail, Report, ResultExt};
use rugpi_cli::info;
use tokio::fs;

pub mod case;
pub mod qemu;

reportify::new_whatever_type! {
    RugpiTestError
}

pub type RugpiTestResult<T> = Result<T, Report<RugpiTestError>>;

/// Command line arguments.
#[derive(Debug, Parser)]
pub struct Args {
    /// Test case file.
    case: PathBuf,
}

#[tokio::main]
pub async fn main() -> RugpiTestResult<()> {
    rugpi_cli::init();
    let args = Args::parse();

    let case = toml::from_str::<TestCase>(
        &fs::read_to_string(&args.case)
            .await
            .whatever("unable to read test case")?,
    )
    .whatever("unable to parse test case")?;

    let vm = qemu::start(&case.vm).await?;

    info!("VM started");

    for step in &case.steps {
        match step {
            case::TestStep::Reboot => todo!(),
            case::TestStep::Copy { .. } => todo!(),
            case::TestStep::Run {
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
            TestStep::Wait { duration_secs } => {
                info!("waiting for {duration_secs} seconds");
                tokio::time::sleep(Duration::from_secs_f64(*duration_secs)).await;
            }
        }
    }
    Ok(())
}
