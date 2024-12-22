//! Functionality for rendering error reports.

use std::{fmt, panic::Location};

use console::style;

use crate::{backtrace::BacktraceImpl, AnyReport, Printable};

trait TreeNode {
    fn render(&self, renderer: &mut Renderer) -> fmt::Result;

    fn style(&self) -> TreeNodeStyle {
        TreeNodeStyle::Simple
    }
}

enum TreeNodeStyle {
    Simple,
    Arrow,
}

pub struct Renderer {
    buffer: String,
    indent: String,
    backtrace_count: u32,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            indent: String::new(),
            backtrace_count: 0,
        }
    }

    fn render_tree<N, I>(&mut self, nodes: I) -> fmt::Result
    where
        I: IntoIterator<Item = N>,
        N: TreeNode,
    {
        let mut nodes = nodes.into_iter().peekable();
        while let Some(node) = nodes.next() {
            let previous_indent = self.indent.len();
            self.buffer.push('\n');
            self.buffer.push_str(&self.indent);
            match node.style() {
                TreeNodeStyle::Simple => {
                    if nodes.peek().is_none() {
                        self.buffer.push_str("╰╴");
                        self.indent.push_str("  ");
                    } else {
                        self.buffer.push_str("├╴");
                        self.indent.push_str("│ ");
                    }
                }
                TreeNodeStyle::Arrow => {
                    if nodes.peek().is_none() {
                        self.buffer.push_str("│   ");
                        self.buffer.push('\n');
                        self.buffer.push_str(&self.indent);
                        self.buffer.push_str("╰─▶ ");
                        self.indent.push_str("    ");
                    } else {
                        self.buffer.push_str("│   ");
                        self.buffer.push('\n');
                        self.buffer.push_str(&self.indent);
                        self.buffer.push_str("├─▶ ");
                        self.indent.push_str("│   ");
                    }
                }
            };
            node.render(self)?;
            if matches!(node.style(), TreeNodeStyle::Arrow) && nodes.peek().is_some() {
                self.buffer.push('\n');
                self.buffer.push_str(&self.indent);
            }
            self.indent.truncate(previous_indent);
        }
        Ok(())
    }

    pub fn into_string(self) -> String {
        self.buffer
    }

    pub fn put_char(&mut self, c: char) {
        if c == '\n' {
            self.buffer.push('\n');
            self.buffer.push_str(&self.indent);
        } else {
            self.buffer.push(c);
        }
    }
}

impl fmt::Write for Renderer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buffer.reserve(s.len());
        for c in s.chars() {
            self.put_char(c);
        }
        Ok(())
    }
}

enum ReportTreeNode<'report> {
    Location {
        location: &'report Location<'static>,
    },
    Info {
        info: &'report dyn Printable,
    },
    Backtrace,
    Cause {
        cause: &'report dyn AnyReport,
    },
}

impl TreeNode for ReportTreeNode<'_> {
    fn render(&self, renderer: &mut Renderer) -> fmt::Result {
        use std::fmt::Write;
        match self {
            ReportTreeNode::Location { location } => {
                write!(
                    renderer,
                    "{}",
                    style(format_args!(
                        "at {}:{}:{}",
                        location.file(),
                        location.line(),
                        location.column()
                    ))
                    .black()
                )
            }
            ReportTreeNode::Cause { cause } => render_report_tree(renderer, *cause),
            ReportTreeNode::Info { info: context } => {
                write!(renderer, "{context}")
            }
            ReportTreeNode::Backtrace => {
                renderer.backtrace_count += 1;
                let number = renderer.backtrace_count;
                write!(renderer, "BACKTRACE ({number})")
            }
        }
    }

    fn style(&self) -> TreeNodeStyle {
        match self {
            ReportTreeNode::Cause { .. } => TreeNodeStyle::Arrow,
            _ => TreeNodeStyle::Simple,
        }
    }
}

fn render_report_tree(renderer: &mut Renderer, report: &dyn AnyReport) -> fmt::Result {
    use std::fmt::Write;
    if let Some(description) = &report.meta().description {
        write!(renderer, "{}", style(description).bold().red())?;
    } else if let Some(description) = report.error().description() {
        write!(renderer, "{}", style(description).bold().red())?;
    } else {
        write!(renderer, "{}", style(report.error().name()).bold().red())?;
    }
    let meta = report.meta();
    renderer.render_tree(
        meta.location
            .iter()
            .map(|location| ReportTreeNode::Location { location })
            .chain(
                meta.info
                    .iter()
                    .rev()
                    .map(|info| ReportTreeNode::Info { info }),
            )
            .chain(meta.backtrace.iter().filter_map(|backtrace| {
                if backtrace.captured() {
                    Some(ReportTreeNode::Backtrace)
                } else {
                    None
                }
            }))
            .chain(meta.causes.iter().map(|cause| ReportTreeNode::Cause {
                cause: &*cause.inner,
            })),
    )?;
    Ok(())
}

fn render_backtraces(renderer: &mut Renderer, report: &dyn AnyReport) -> fmt::Result {
    use std::fmt::Write;
    if let Some(backtrace) = &report.meta().backtrace {
        if backtrace.captured() {
            renderer.backtrace_count += 1;
            let number = renderer.backtrace_count;
            write!(renderer, "\n\n\n━━━━ BACKTRACE ({number})\n\n")?;
            backtrace.render(renderer)?;
        }
    }
    for cause in &report.meta().causes {
        render_backtraces(renderer, &*cause.inner)?;
    }
    Ok(())
}

pub fn render_report(report: &dyn AnyReport) -> String {
    let mut renderer = Renderer::new();
    render_report_tree(&mut renderer, report).unwrap();
    renderer.backtrace_count = 0;
    render_backtraces(&mut renderer, report).unwrap();
    renderer.into_string()
}
