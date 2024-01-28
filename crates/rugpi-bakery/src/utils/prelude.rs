//! Custom prelude.

pub use anyhow::{anyhow, bail};
pub use rugpi_common::Anyhow;
pub use tracing::{debug, error, info, trace, warn};

pub use super::{logging::bug, once_cell_ext::OnceCellExt};
