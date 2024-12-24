use std::io;

/// Re-export tracing macros.
pub use tracing::{debug, error, info, trace, warn};

/// Initialize logging and other CLI related functionality.
pub fn init() {
    let format = tracing_subscriber::fmt::format()
        .without_time()
        .with_target(false)
        .compact();
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .event_format(format)
        .init();
}
