//! Widgets for building rich CLI experiences.
//!
//! A _widget_ is a reusable component that can be drawn on the terminal screen. Widgets
//! must implement the [`Widget`] trait. This trait requires implementing a method,
//! [`draw`][Widget::draw], which takes a widget and draws it into a given _drawing
//! context_ ([`DrawCtx`]). In addition to providing a buffer for drawing, the context can
//! also be queried for the dimensions and capabilities of the terminal.
//!
//! This module contains a collection of prebuilt, general-purpose widgets for building
//! rich CLI experiences.

use std::cmp::Ordering;
use std::fmt::Display;

use crate::style::{Style, Styled};
use crate::{DrawCtx, VisualWidth};

/// Widget that can be drawn into [`DrawCtx`].
pub trait Widget {
    /// Draw the widget into the provided [`DrawCtx`].
    fn draw(self, ctx: &mut DrawCtx);

    /// Wraps the widget in [`Styled`].
    fn styled(self) -> Styled<Self>
    where
        Self: Sized,
    {
        Styled::new(self)
    }
}

impl Widget for &str {
    fn draw(self, ctx: &mut DrawCtx) {
        ctx.write_str(self);
    }
}

impl Widget for &String {
    fn draw(self, ctx: &mut DrawCtx) {
        ctx.write_str(self);
    }
}

impl<W: Widget> Widget for Styled<W> {
    fn draw(self, ctx: &mut DrawCtx) {
        ctx.with_style(self.style, |ctx| self.value.draw(ctx));
    }
}

/// Progress bar.
#[derive(Debug)]
pub struct ProgressBar {
    position: u64,
    length: u64,
    width_limit: Option<VisualWidth>,
    percentage_style: Option<Style>,
    show_percentage: bool,
}

impl ProgressBar {
    /// Symbols for rendering a Unicode progress bar.
    const UNICODE_SYMBOLS: &[char] = &['█', '█', '▉', '▊', '▋', '▌', '▍', '▎', '▏', ' '];
    /// Symbols for rendering an ASCII progress bar.
    const ASCII_SYMBOLS: &[char] = &['=', '>', ' '];

    /// Create a new progress bar.
    pub fn new(position: u64, length: u64) -> Self {
        assert!(length > 0, "length must be greater than zero");
        Self {
            position,
            length,
            percentage_style: None,
            width_limit: None,
            show_percentage: true,
        }
    }

    /// Set the style of the percentage indicator.
    pub fn percentage_style(mut self, style: Style) -> Self {
        self.percentage_style = Some(style);
        self
    }

    /// Limit the width of the progress bar.
    pub fn limit_width(mut self, width_limit: VisualWidth) -> Self {
        self.width_limit = Some(width_limit);
        self
    }

    /// Hide the percentage indicator.
    pub fn hide_percentage(mut self) -> Self {
        self.show_percentage = false;
        self
    }
}

impl Widget for &ProgressBar {
    fn draw(self, ctx: &mut DrawCtx) {
        let mut width = ctx.measure_remaining_width();
        if let Some(width_limit) = self.width_limit {
            width = width.min(width_limit);
        }
        ctx.with_optional_style(self.percentage_style, |ctx| {
            if self.show_percentage && width >= 7 {
                let per_mille = (1000 * self.position / self.length).min(1000);
                let percentage = per_mille / 10;
                let fraction = per_mille % 10;
                write!(ctx, "{percentage:>3}.{fraction}% ");
                width -= 7;
            }
        });
        if width < 2 {
            // Do not draw the progress bar if less than 2 cells are remaining.
            return;
        }
        let symbols = if ctx.supports_unicode() {
            ProgressBar::UNICODE_SYMBOLS
        } else {
            width -= 2;
            ctx.write_char('[');
            ProgressBar::ASCII_SYMBOLS
        };
        let variants = (symbols.len() - 1) as u64;
        let mut remaining = width * self.position * variants / self.length;
        for pos in width.iter() {
            match remaining.cmp(&variants.into()) {
                Ordering::Greater => ctx.write_char(symbols[0]),
                Ordering::Equal => {
                    if pos == width - 1 {
                        ctx.write_char(symbols[0])
                    } else {
                        ctx.write_char(symbols[1])
                    }
                }
                Ordering::Less => {
                    ctx.write_char(symbols[(variants - remaining.into_u64()) as usize])
                }
            }
            remaining -= variants;
        }
        if !ctx.supports_unicode() {
            ctx.write_char(']');
        }
    }
}

/// Progress spinner.
#[derive(Debug)]
pub struct ProgressSpinner(());

impl ProgressSpinner {
    /// Period of the spinner in milliseconds.
    const PERIOD_MS: u128 = 1_000;

    /// Symbols for rendering a Unicode spinner.
    const UNICODE_SYMBOLS: &[char] = &['◐', '◓', '◑', '◒'];
    /// Symbols for rendering an ASCII spinner.
    const ASCII_SYMBOLS: &[char] = &['|', '/', '-', '\\'];

    /// Create a new spinner.
    pub fn new() -> Self {
        Self(())
    }
}

impl Widget for &ProgressSpinner {
    fn draw(self, ctx: &mut DrawCtx) {
        let position = (ctx.time().as_millis() % ProgressSpinner::PERIOD_MS) as u64;
        let symbols = if ctx.supports_unicode() {
            ProgressSpinner::UNICODE_SYMBOLS
        } else {
            ProgressSpinner::ASCII_SYMBOLS
        };
        let symbol_idx = position * (symbols.len() as u64) / (ProgressSpinner::PERIOD_MS as u64);
        ctx.write_char(symbols[symbol_idx as usize]);
    }
}

/// Horizontal rule.
#[derive(Debug)]
pub struct Rule {
    width_limit: Option<VisualWidth>,
}

impl Rule {
    /// Create a new horizontal rule.
    pub fn new() -> Self {
        Self { width_limit: None }
    }

    /// Limit the width of the rule.
    pub fn limit_width(mut self, width_limit: VisualWidth) -> Self {
        self.width_limit = Some(width_limit);
        self
    }
}

impl Widget for &Rule {
    fn draw(self, ctx: &mut DrawCtx) {
        let mut width = ctx.measure_remaining_width();
        if let Some(width_limit) = self.width_limit {
            width = width.min(width_limit);
        }
        for _ in width.iter() {
            if ctx.supports_unicode() {
                ctx.write_char('━');
            } else {
                ctx.write_char('-');
            }
        }
    }
}

/// Text consisting of a sequence of lines.
#[derive(Debug)]
pub struct Text<'cx, I> {
    lines: I,
    prefix: &'cx str,
    prefix_style: Option<Style>,
    suffix: &'cx str,
    suffix_style: Option<Style>,
    truncated: &'cx str,
    truncated_style: Option<Style>,
}

impl<'cx, I> Text<'cx, I> {
    /// Create a new text area.
    pub fn new(lines: I) -> Self {
        Self {
            lines,
            prefix: "",
            prefix_style: None,
            suffix: "",
            suffix_style: None,
            truncated: "...",
            truncated_style: None,
        }
    }

    /// Set the prefix to render before each line.
    pub fn prefix(mut self, prefix: &'cx str) -> Self {
        self.prefix = prefix;
        self
    }

    /// Set the style of the prefix.
    pub fn prefix_style(mut self, style: Style) -> Self {
        self.prefix_style = Some(style);
        self
    }

    /// Set the suffix to render after each line.
    pub fn suffix(mut self, suffix: &'cx str) -> Self {
        self.suffix = suffix;
        self
    }

    /// Set the style of the suffix.
    pub fn suffix_style(mut self, style: Style) -> Self {
        self.suffix_style = Some(style);
        self
    }

    /// Set the truncation indicator.
    pub fn truncated(mut self, truncated: &'cx str) -> Self {
        self.truncated = truncated;
        self
    }

    /// Set the style of the truncation indicator.
    pub fn truncated_style(mut self, style: Style) -> Self {
        self.truncated_style = Some(style);
        self
    }
}

impl<I> Widget for Text<'_, I>
where
    I: IntoIterator,
    <I as IntoIterator>::Item: AsRef<str>,
{
    fn draw(self, ctx: &mut crate::DrawCtx) {
        ctx.start_line();
        let max_line_width = ctx.available_width()
            - VisualWidth::measure(self.prefix)
            - VisualWidth::measure(self.suffix)
            - VisualWidth::measure(self.truncated);
        for (idx, line) in self.lines.into_iter().enumerate() {
            // Strip any formatting from the provided line.
            let line = console::strip_ansi_codes(line.as_ref().trim_end());
            let line_width = VisualWidth::measure(&line);
            if idx > 0 {
                // Terminate the previous line with a newline.
                ctx.write_char('\n');
            }
            ctx.with_optional_style(self.prefix_style, |ctx| {
                ctx.write_str(self.prefix);
            });
            for c in line.chars().take(max_line_width.into_u64() as usize) {
                if c.is_control() {
                    continue;
                } else if c == '\t' {
                    ctx.write_char(' ');
                } else {
                    ctx.write_char(c);
                }
            }
            if line_width > max_line_width {
                ctx.with_optional_style(self.truncated_style, |ctx| {
                    ctx.write_str(self.truncated);
                });
            } else {
                for _ in (max_line_width - line_width).iter() {
                    ctx.write_char(' ');
                }
            }
            ctx.with_optional_style(self.suffix_style, |ctx| {
                ctx.write_str(self.suffix);
            });
        }
        ctx.fill_line();
    }
}

/// Heading.
#[derive(Debug)]
pub struct Heading<H> {
    heading: H,
    decoration_style: Option<Style>,
}

impl<H> Heading<H> {
    /// Create a new heading.
    pub fn new(heading: H) -> Self {
        Self {
            heading,
            decoration_style: None,
        }
    }

    /// Set the style of the decoration.
    pub fn decoration_style(mut self, style: Style) -> Self {
        self.decoration_style = Some(style);
        self
    }
}

impl<H: Display> Widget for Heading<H> {
    fn draw(self, ctx: &mut DrawCtx) {
        let decoration_char = if ctx.supports_unicode() { '━' } else { '-' };
        ctx.with_optional_style(self.decoration_style, |ctx| {
            ctx.write_repeated(decoration_char, 2.into());
        });
        ctx.write_char(' ');
        ctx.with_style(Style::new().bold(), |ctx| {
            write!(ctx, "{}", self.heading);
        });
        let mut remaining = ctx.measure_remaining_width();
        if !remaining.is_zero() {
            ctx.with_optional_style(self.decoration_style, |ctx| {
                ctx.write_char(' ');
                remaining -= 1;
                ctx.write_repeated(decoration_char, remaining);
            });
        }
        ctx.write_char('\n');
    }
}
