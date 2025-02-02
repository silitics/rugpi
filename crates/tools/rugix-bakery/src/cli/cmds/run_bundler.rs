//! The `bundler` command.

use std::ffi::CString;
use std::ops::Deref;

use reportify::ResultExt;

use crate::cli::args::BundlerCommand;
use crate::BakeryResult;

/// Run the `bundler` command.
pub fn run(cmd: &BundlerCommand) -> BakeryResult<()> {
    let mut args = vec![c"/usr/local/bin/rugix-bundler".to_owned()];
    for arg in &cmd.args {
        args.push(CString::new(arg.as_bytes()).unwrap());
    }
    let args = args.iter().map(|arg| arg.deref()).collect::<Vec<_>>();
    // Replace ourselves with Rugix Bundler.
    nix::unistd::execv::<&std::ffi::CStr>(c"/usr/local/bin/rugix-bundler", &args)
        .whatever("error executing Rugix Bundler")?;
    Ok(())
}
