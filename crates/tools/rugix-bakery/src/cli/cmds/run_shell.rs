//! The `shell` command.

use reportify::ResultExt;

use crate::BakeryResult;

/// Run the `shell` command.
pub fn run() -> BakeryResult<()> {
    // Replace ourselves with a shell. This is primarily intended for debugging.
    nix::unistd::execv::<&std::ffi::CStr>(c"/bin/zsh", &[]).whatever("error executing shell")?;
    Ok(())
}
