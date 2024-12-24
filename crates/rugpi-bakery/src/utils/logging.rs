#[macro_export]
macro_rules! bug {
    ($msg:literal) => {
        rugpi_cli::error!("[BUG] {}", $msg)
    };
}

pub use bug;
