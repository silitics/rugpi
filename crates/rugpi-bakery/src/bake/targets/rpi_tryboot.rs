use std::path::Path;

use rugpi_common::{fsutils::copy_recursive, Anyhow};
use xscript::{run, Run};

pub fn initialize_tryboot(config_dir: &Path, boot_dir: &Path, root_dir: &Path) -> Anyhow<()> {
    copy_recursive(root_dir.join("boot"), &boot_dir)?;
    run!(["rm", "-rf", root_dir.join("boot")])?;
    std::fs::create_dir_all(root_dir.join("boot"))?;
    copy_recursive("/usr/share/rugpi/boot/tryboot", &config_dir)?;
    Ok(())
}
