use std::path::Path;

use reportify::ResultExt;

use rugix_common::fsutils::copy_recursive;

use crate::BakeryResult;

pub fn initialize_tryboot(config_dir: &Path) -> BakeryResult<()> {
    copy_recursive("/usr/share/rugix/boot/tryboot", &config_dir)
        .whatever("unable to initialize tryboot")?;
    Ok(())
}
