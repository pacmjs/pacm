use crossterm::{ExecutableCommand, cursor, terminal};
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

pub struct Logger {
    start_time: Instant,
    quiet: bool,
    current_line: Arc<Mutex<String>>,
}

pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
    Debug,
    Shell,
}

impl Logger {
    #[must_use]
    pub fn new(quiet: bool) -> Self {
        Self {
            start_time: Instant::now(),
            quiet,
            current_line: Arc::new(Mutex::new(String::new())),
        }
    }

    fn clear_current_line(&self) {
        if self.quiet {
            return;
        }

        let mut stdout = io::stdout();
        let _ = stdout.execute(cursor::MoveToColumn(0));
        let _ = stdout.execute(terminal::Clear(terminal::ClearType::CurrentLine));
        let _ = stdout.flush();
    }

    pub fn update_line(&self, message: &str) {
        if self.quiet {
            return;
        }

        self.clear_current_line();
        print!("{message}");
        let _ = io::stdout().flush();

        if let Ok(mut line) = self.current_line.lock() {
            *line = message.to_string();
        }
    }

    pub fn finish_line(&self, message: &str) {
        if self.quiet {
            return;
        }

        self.clear_current_line();
        println!("{message}");

        if let Ok(mut line) = self.current_line.lock() {
            line.clear();
        }
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        if self.quiet && !matches!(level, LogLevel::Error) {
            return;
        }

        self.clear_current_line();

        let (prefix, colored_message) = match level {
            LogLevel::Info => (
                "pacm".bright_cyan().bold().to_string(),
                message.white().to_string(),
            ),
            LogLevel::Success => (
                "✓".bright_green().bold().to_string(),
                message.bright_green().to_string(),
            ),
            LogLevel::Warning => (
                "⚠".bright_yellow().bold().to_string(),
                message.bright_yellow().to_string(),
            ),
            LogLevel::Error => (
                "✗".bright_red().bold().to_string(),
                message.bright_red().to_string(),
            ),
            LogLevel::Debug => (
                "•".bright_black().bold().to_string(),
                message.bright_black().to_string(),
            ),
            LogLevel::Shell => (
                "$".bright_blue().bold().to_string(),
                message.bright_black().to_string(),
            ),
        };

        println!("{prefix} {colored_message}");

        if let Ok(mut line) = self.current_line.lock() {
            line.clear();
        }
    }

    pub fn finish(&self, message: &str) {
        let elapsed = self.start_time.elapsed();
        let time_str = if elapsed.as_millis() < 1000 {
            format!("{}ms", elapsed.as_millis())
        } else {
            format!("{:.2}s", elapsed.as_secs_f64())
        };

        let final_message = format!(
            "{} {} {}",
            "✓".bright_green().bold(),
            message.bright_green(),
            format!("[{time_str}]").bright_black()
        );

        self.finish_line(&final_message);
    }
    pub fn progress(&self, message: &str, current: usize, total: usize) {
        if self.quiet {
            return;
        }

        let spinners = ["◐", "◓", "◑", "◒"];
        let spinner = spinners.get(current % spinners.len()).unwrap_or(&"◐");

        let progress_text = if total > 0 {
            format!(
                "{} {} ({}/{})",
                spinner.bright_cyan(),
                message.bright_white(),
                current.to_string().bright_cyan().bold(),
                total.to_string().bright_white()
            )
        } else {
            format!("{} {}", spinner.bright_cyan(), message.bright_white())
        };

        self.update_line(&progress_text);
    }

    pub fn status(&self, message: &str) {
        if self.quiet {
            return;
        }

        let status_msg = format!("{} {}", "◦".bright_cyan(), message.bright_white());
        self.update_line(&status_msg);
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn success(&self, message: &str) {
        self.log(LogLevel::Success, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn debug(&self, message: &str, debug_enabled: bool) {
        if debug_enabled {
            self.log(LogLevel::Debug, message);
        }
    }

    pub fn shell(&self, command: &str) {
        self.log(LogLevel::Shell, command);
    }
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_logger(quiet: bool) {
    let _ = LOGGER.set(Logger::new(quiet));
}

fn get_logger() -> &'static Logger {
    LOGGER
        .get()
        .unwrap_or_else(|| panic!("Logger not initialized. Call init_logger() first."))
}

pub fn update_line(message: &str) {
    get_logger().update_line(message);
}

pub fn status(message: &str) {
    get_logger().status(message);
}

pub fn info(message: &str) {
    get_logger().info(message);
}

pub fn success(message: &str) {
    get_logger().success(message);
}

pub fn warn(message: &str) {
    get_logger().warn(message);
}

pub fn error(message: &str) {
    get_logger().error(message);
}

pub fn debug(message: &str, debug_enabled: bool) {
    get_logger().debug(message, debug_enabled);
}

pub fn shell(command: &str) {
    get_logger().shell(command);
}

pub fn progress(message: &str, current: usize, total: usize) {
    get_logger().progress(message, current, total);
}

pub fn finish(message: &str) {
    get_logger().finish(message);
}

pub fn finish_line(message: &str) {
    get_logger().finish_line(message);
}
