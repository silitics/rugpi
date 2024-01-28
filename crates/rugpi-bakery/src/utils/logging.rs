pub fn init_logging() {
    tracing_subscriber::fmt::init();
}

#[macro_export]
macro_rules! bug {
    ($msg:literal) => {
        tracing::error!("[BUG] {}", $msg)
    };
}

pub use bug;
