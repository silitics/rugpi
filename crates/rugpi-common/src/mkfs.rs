use xscript::{run, Run};

pub fn make_boot_fs(dev: impl AsRef<str>, label: impl AsRef<str>) -> anyhow::Result<()> {
    run!(["mkfs.vfat", "-n", label, dev])?;
    Ok(())
}

pub fn make_system_fs(dev: impl AsRef<str>, label: impl AsRef<str>) -> anyhow::Result<()> {
    run!(["mkfs.ext4", "-F", "-L", label, dev])?;
    Ok(())
}
