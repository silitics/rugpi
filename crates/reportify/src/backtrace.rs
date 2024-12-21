//! Functionality for capturing and rendering backtraces.

use std::{env, fmt, sync::atomic};

use console::style;

use crate::renderer::Renderer;

/// Backtrace implementation.
pub(crate) trait BacktraceImpl {
    /// Capture a backtrace, if backtraces are enabled.
    fn capture() -> Self;

    /// Forcibly capture a backtrace even if backtraces are not enabled.
    fn force_capture() -> Self;

    /// Indicates whether a backtrace has been captured.
    fn captured(&self) -> bool;

    /// Render the backtrace.
    fn render(&self, renderer: &mut Renderer) -> fmt::Result;
}

impl BacktraceImpl for std::backtrace::Backtrace {
    fn capture() -> Self {
        std::backtrace::Backtrace::capture()
    }

    fn force_capture() -> Self {
        std::backtrace::Backtrace::force_capture()
    }

    fn captured(&self) -> bool {
        matches!(self.status(), std::backtrace::BacktraceStatus::Captured)
    }

    fn render(&self, renderer: &mut Renderer) -> fmt::Result {
        use std::fmt::Write;
        write!(renderer, "{self}")
    }
}

#[cfg(feature = "backtrace")]
impl BacktraceImpl for Option<backtrace::Backtrace> {
    fn capture() -> Self {
        if backtrace_enabled() {
            Self::force_capture()
        } else {
            None
        }
    }

    fn force_capture() -> Self {
        Some(backtrace::Backtrace::new())
    }

    fn captured(&self) -> bool {
        self.is_some()
    }

    fn render(&self, renderer: &mut Renderer) -> fmt::Result {
        use std::fmt::Write;

        #[derive(Clone, Copy, Debug)]
        enum FramesSegment {
            Backtrace,
            Reportify,
            User,
        }

        match self {
            Some(backtrace) => {
                let mut segment = FramesSegment::Backtrace;
                'frame: for (frame_idx, frame) in backtrace.frames().iter().enumerate() {
                    let symbols = frame.symbols();
                    // XXX: Some crude logic to skip frames based on symbol names. This should be
                    // improved in the future. Have a look at how `color-eyre` does it.
                    for symbol in symbols {
                        if let Some(name) = symbol.name() {
                            // spell-checker:ignore demangle
                            // We need to demangle the symbol.
                            let name = name.to_string();
                            match segment {
                                FramesSegment::Backtrace => {
                                    if name.starts_with("reportify") {
                                        segment = FramesSegment::Reportify;
                                    }
                                    continue 'frame;
                                }
                                FramesSegment::Reportify => {
                                    if !name.contains("reportify") {
                                        segment = FramesSegment::User;
                                        writeln!(
                                            renderer,
                                            "   ⋮  skipped {} frames\n",
                                            frame_idx - 1
                                        )?;
                                    } else {
                                        continue 'frame;
                                    }
                                }
                                FramesSegment::User => {
                                    if name.starts_with(
                                        "std::sys::backtrace::__rust_begin_short_backtrace",
                                    ) {
                                        writeln!(
                                            renderer,
                                            "\n   ⋮  skipped {} frames",
                                            backtrace.frames().len() - frame_idx
                                        )?;
                                        break 'frame;
                                    }
                                }
                            }
                        }
                    }
                    if symbols.is_empty() {
                        writeln!(renderer, "{frame_idx:>4}: {:?}", frame.ip())?;
                    } else {
                        for (symbol_idx, symbol) in frame.symbols().iter().enumerate() {
                            if symbol_idx == 0 {
                                write!(renderer, "{frame_idx:>4}:")?;
                            } else {
                                write!(renderer, "     ")?;
                            }
                            if let Some(name) = symbol.name() {
                                write!(renderer, " {}", style(format_args!("{name:#}")).cyan())?;
                            }
                            if let Some(addr) = symbol.addr() {
                                write!(renderer, " {}", style(format_args!("({addr:?})")).black())?;
                            }
                            if let Some(file) = symbol.filename() {
                                write!(renderer, "\n      at {}", file.to_string_lossy())?;
                                if let Some(line) = symbol.lineno() {
                                    write!(renderer, ":{line}")?;
                                    if let Some(column) = symbol.colno() {
                                        write!(renderer, ":{column}")?;
                                    }
                                }
                            }
                            writeln!(renderer)?;
                        }
                    }
                }
                Ok(())
            }
            None => write!(renderer, "<no backtrace>"),
        }
    }
}

/// Wrapper to hide the concrete type of the backtrace implementation.
pub(crate) struct BacktraceWrapped<B> {
    backtrace: B,
}

impl<B: BacktraceImpl> BacktraceImpl for BacktraceWrapped<B> {
    fn capture() -> Self {
        Self {
            backtrace: B::capture(),
        }
    }

    fn force_capture() -> Self {
        Self {
            backtrace: B::force_capture(),
        }
    }

    fn captured(&self) -> bool {
        self.backtrace.captured()
    }

    fn render(&self, renderer: &mut Renderer) -> fmt::Result {
        self.backtrace.render(renderer)
    }
}

/// Backtrace.
#[cfg(not(feature = "backtrace"))]
pub(crate) type Backtrace = BacktraceWrapped<std::backtrace::Backtrace>;

/// Backtrace.
#[cfg(feature = "backtrace")]
pub(crate) type Backtrace = BacktraceWrapped<Option<backtrace::Backtrace>>;

/// Indicates whether backtraces should be captured.
fn backtrace_enabled() -> bool {
    // This code has been taken from the standard library's `backtrace.rs`. The result is
    // cached to avoid querying the environment every time which can be slow.
    static ENABLED: atomic::AtomicU8 = atomic::AtomicU8::new(0);
    match ENABLED.load(atomic::Ordering::Relaxed) {
        0 => {}
        1 => return false,
        _ => return true,
    }
    let enabled = match env::var("RUST_LIB_BACKTRACE") {
        Ok(s) => s != "0",
        Err(_) => match env::var("RUST_BACKTRACE") {
            Ok(s) => s != "0",
            Err(_) => false,
        },
    };
    ENABLED.store(enabled as u8 + 1, atomic::Ordering::Relaxed);
    enabled
}
