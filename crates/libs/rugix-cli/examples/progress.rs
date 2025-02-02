use std::collections::VecDeque;
use std::fmt::Display;
use std::sync::Mutex;
use std::time::Duration;

use tracing::info;

use rugix_cli::style::{Styled, Stylize};
use rugix_cli::widgets::{Heading, ProgressBar, ProgressSpinner, Rule, Text, Widget};
use rugix_cli::{DrawCtx, StatusSegment};

pub fn cli() {
    let _hello_world = rugix_cli::add_status(Styled::new("Hello World!").blue());
    let task_progress = rugix_cli::add_status(TaskProgress {
        state: Mutex::new(TaskState {
            current_step: 0,
            total_steps: 1000,
            title: String::new(),
            message: String::new(),
            log_lines: VecDeque::new(),
        }),
    });
    let _rugix_banner = rugix_cli::add_status(
        r#"
         ____                    _           _
        |  _ \ _   _  __ _ _ __ (_)   __   _/ |
        | |_) | | | |/ _` | '_ \| |   \ \ / / |
        |  _ <| |_| | (_| | |_) | |    \ V /| |
        |_| \_\\__,_|\__, | .__/|_|     \_/ |_|     by Silitics
                    |___/|_|

        For more information, visit https://oss.silitics.com/rugpi/.
        "#,
    );

    for step in 0..=1000 {
        std::thread::sleep(Duration::from_millis(10));
        task_progress.set_step(step);
        if step % 10 == 0 {
            task_progress.push_log_line(format!("Position {step} reached!"));
        }
        if step % 100 == 0 {
            info!("Position {step} reached!");
        }
        if step % 400 == 0 {
            task_progress.set_message("Doing work...");
        } else if step % 400 == 200 {
            task_progress.set_message("");
        }
        if step == 500 {
            task_progress.set_title("Reached 500");
        }
        rugix_cli::redraw();
    }

    rugix_cli::hide_status();

    std::thread::sleep(Duration::from_secs(2));

    rugix_cli::show_status();

    std::thread::sleep(Duration::from_secs(5));
}

pub fn main() {
    rugix_cli::CliBuilder::new().init();
    cli();
    rugix_cli::force_redraw();
}

struct TaskProgress {
    state: Mutex<TaskState>,
}

impl TaskProgress {
    pub fn set_step(&self, step: u64) {
        self.state.lock().unwrap().current_step = step;
    }

    pub fn set_title<D: Display>(&self, title: D) {
        self.state.lock().unwrap().title = title.to_string();
    }

    pub fn set_message<D: Display>(&self, message: D) {
        self.state.lock().unwrap().message = message.to_string();
    }

    pub fn push_log_line(&self, line: String) {
        let mut state = self.state.lock().unwrap();
        state.log_lines.push_back(line);
        while state.log_lines.len() > 5 {
            state.log_lines.pop_front();
        }
    }
}

struct TaskState {
    current_step: u64,
    total_steps: u64,
    title: String,
    message: String,
    log_lines: VecDeque<String>,
}

impl StatusSegment for TaskProgress {
    fn draw(&self, ctx: &mut DrawCtx) {
        let state = self.state.lock().unwrap();
        if state.title.is_empty() {
            Rule::new().draw(ctx)
        } else {
            Heading::new(&state.title).draw(ctx);
        };
        ProgressSpinner::new().draw(ctx);
        write!(ctx, " Step [{}/{}] ", state.current_step, state.total_steps);
        if !state.message.is_empty() {
            ctx.write_str(&state.message);
            ctx.write_char(' ');
        }
        ProgressBar::new(state.current_step, state.total_steps).draw(ctx);
        let remaining_height = ctx.measure_remaining_height();
        if !state.log_lines.is_empty() && remaining_height > 2 {
            let show_lines = state
                .log_lines
                .len()
                .min(5)
                .min(remaining_height.into_u64() as usize);
            let skip_lines = state.log_lines.len() - show_lines;
            Text::new(state.log_lines.iter().skip(skip_lines))
                .prefix("> ")
                .styled()
                .black()
                .draw(ctx);
        }
    }
}
