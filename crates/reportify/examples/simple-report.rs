use std::io;
use std::path::Path;

use reportify::{new_whatever_type, Report, ResultExt};

new_whatever_type! {
    AppError
}

new_whatever_type! {
    ConfigError
}

fn read_config(path: &Path) -> Result<u64, Report<ConfigError>> {
    let config = std::fs::read_to_string(path).whatever_with(|error| match error.kind() {
        io::ErrorKind::NotFound => "configuration file not found",
        _ => "unable to read configuration file",
    })?;
    config
        .trim()
        .parse()
        .whatever("unable to parse configuration file")
        .with_info(|_| format!("config: {:?}", config.trim()))
}

pub fn run() -> Result<(), Report<AppError>> {
    let path = "path/does/not/exist.toml".as_ref();
    read_config(path)
        .whatever("unable to load configuration")
        .with_info(|_| format!("path: {path:?}"))?;
    Ok(())
}

pub fn main() {
    if let Err(report) = run() {
        eprintln!("\n{report:?}");
        std::process::exit(1)
    }
}
