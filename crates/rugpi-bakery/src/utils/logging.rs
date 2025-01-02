#[macro_export]
macro_rules! bug {
    ($msg:literal) => {
        tracing::error!("[BUG] {}", $msg)
    };
}

pub use bug;
