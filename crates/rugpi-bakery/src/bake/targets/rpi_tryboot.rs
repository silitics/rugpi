use std::path::Path;

use rugpi_common::{fsutils::copy_recursive, Anyhow};

pub fn initialize_tryboot(config_dir: &Path) -> Anyhow<()> {
    copy_recursive("/usr/share/rugpi/boot/tryboot", &config_dir)?;
    Ok(())
}
