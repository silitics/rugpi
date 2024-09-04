use super::{boot_entries::BootEntryIdx, System};
use crate::Anyhow;

/// Implementation of a boot flow.
pub trait BootFlow {
    /// Set the entry to try on the next boot.
    ///
    /// If booting fails, the bootloader should fallback to the previous default.
    ///
    /// Note that this function may change the default entry.
    fn set_try_next(&self, system: &System, entry: BootEntryIdx) -> Anyhow<()>;

    /// Set the default entry.
    fn set_default(&self, system: &System, entry: BootEntryIdx) -> Anyhow<()>;

    /// Get the default entry.
    fn get_default(&self, system: &System) -> Anyhow<BootEntryIdx>;

    /// Get the number of remaining attempts for the given entry.
    ///
    /// Returns [`None`] in case there is an unlimited number of attempts.
    fn remaining_attempts(&self, system: &System, entry: BootEntryIdx) -> Anyhow<Option<u64>>;

    /// Get the status of the boot entry.
    fn get_status(&self, system: &System, entry: BootEntryIdx) -> Anyhow<BootEntryStatus>;

    /// Mark an entry as good.
    fn mark_good(&self, system: &System, entry: BootEntryIdx) -> Anyhow<()>;

    /// Mark an entry as bad.
    fn mark_bad(&self, system: &System, entry: BootEntryIdx) -> Anyhow<()>;
}

pub enum BootEntryStatus {
    Unknown,
    Good,
    Bad,
}
