pub fn init_logging() {
    let format = tracing_subscriber::fmt::format()
        .without_time()
        .with_target(false)
        .compact();
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .event_format(format)
        .init();
}

#[macro_export]
macro_rules! bug {
    ($msg:literal) => {
        tracing::error!("[BUG] {}", $msg)
    };
}

use std::io;

pub use bug;
