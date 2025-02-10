use std::collections::VecDeque;

use rugix_cli::style::Stylize;
use rugix_cli::widgets::{Heading, Text, Widget};
use rugix_cli::{StatusSegment, VisualHeight};

#[derive(Debug)]
pub struct CliLog {
    state: std::sync::Mutex<CliLogState>,
    title: String,
    line_limit: usize,
}

impl CliLog {
    pub fn new(title: String) -> Self {
        Self {
            state: std::sync::Mutex::default(),
            title,
            line_limit: 15,
        }
    }

    pub fn current_lines(&self) -> String {
        let state = self.state.lock().unwrap();
        let mut lines = String::new();
        for (idx, line) in state.lines.iter().enumerate() {
            if idx > 0 {
                lines.push('\n');
            }
            lines.push_str(line);
        }
        lines
    }

    pub fn push_line(&self, line: String) {
        let mut state = self.state.lock().unwrap();
        state.lines.push_back(line);
        while state.lines.len() > self.line_limit {
            state.lines.pop_front();
        }
    }
}

#[derive(Debug, Default)]
struct CliLogState {
    lines: VecDeque<String>,
}

impl StatusSegment for CliLog {
    fn draw(&self, ctx: &mut rugix_cli::DrawCtx) {
        Heading::new(&self.title).draw(ctx);
        let state = self.state.lock().unwrap();
        let show_lines = VisualHeight::from_usize(state.lines.len())
            .min(ctx.measure_remaining_height())
            .into_u64() as usize;
        let skip_lines = state.lines.len() - show_lines;
        Text::new(state.lines.iter().skip(skip_lines))
            .prefix("> ")
            .styled()
            .dark_gray()
            .draw(ctx);
    }
}
