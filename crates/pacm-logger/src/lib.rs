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
    pub fn new(quiet: bool) -> Self {
        Self {
            start_time: Instant::now(),
            quiet,
            current_line: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Clear the current line and move cursor to beginning
    fn clear_current_line(&self) {
        if self.quiet {
            return;
        }

        let mut stdout = io::stdout();
        let _ = stdout.execute(cursor::MoveToColumn(0));
        let _ = stdout.execute(terminal::Clear(terminal::ClearType::CurrentLine));
        let _ = stdout.flush();
    }

    /// Replace the current line with new content (Bun-style single line updates)
    pub fn update_line(&self, message: &str) {
        if self.quiet {
            return;
        }

        self.clear_current_line();
        print!("{}", message);
        let _ = io::stdout().flush();

        if let Ok(mut line) = self.current_line.lock() {
            *line = message.to_string();
        }
    }

    /// Show a final message and clear the updating line
    pub fn finish_line(&self, message: &str) {
        if self.quiet {
            return;
        }

        self.clear_current_line();
        println!("{}", message);

        if let Ok(mut line) = self.current_line.lock() {
            line.clear();
        }
    }

    /// Print a permanent log message (does not get replaced)
    pub fn log(&self, level: LogLevel, message: &str) {
        if self.quiet && !matches!(level, LogLevel::Error) {
            return;
        }

        // Clear any updating line first
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

        println!("{} {}", prefix, colored_message);

        if let Ok(mut line) = self.current_line.lock() {
            line.clear();
        }
    }

    /// Show the final completion message with elapsed time
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
            format!("[{}]", time_str).bright_black()
        );

        self.finish_line(&final_message);
    }

    /// Show progress with a spinner and counter
    pub fn progress(&self, message: &str, current: usize, total: usize) {
        if self.quiet {
            return;
        }

        // Use different spinner frames for smooth animation
        let spinners = ["◐", "◓", "◑", "◒"];
        let spinner = spinners[current % spinners.len()];

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

    /// Show a simple updating status message
    pub fn status(&self, message: &str) {
        if self.quiet {
            return;
        }

        let status_msg = format!("{} {}", "◦".bright_cyan(), message.bright_white());
        self.update_line(&status_msg);
    }

    // Convenience methods
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

/// Global logger instance using OnceLock for thread-safe initialization
static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Initialize the global logger
pub fn init_logger(quiet: bool) {
    let _ = LOGGER.set(Logger::new(quiet));
}

/// Get the global logger instance
fn get_logger() -> &'static Logger {
    LOGGER
        .get()
        .expect("Logger not initialized. Call init_logger() first.")
}

// Global convenience functions for easy usage throughout the codebase

/// Replace the current line with new content (main function for Bun-style logging)
pub fn update_line(message: &str) {
    get_logger().update_line(message);
}

/// Show a simple status message that updates in place
pub fn status(message: &str) {
    get_logger().status(message);
}

/// Show a permanent info message
pub fn info(message: &str) {
    get_logger().info(message);
}

/// Show a permanent success message
pub fn success(message: &str) {
    get_logger().success(message);
}

/// Show a permanent warning message
pub fn warn(message: &str) {
    get_logger().warn(message);
}

/// Show a permanent error message
pub fn error(message: &str) {
    get_logger().error(message);
}

/// Show a debug message (only if debug mode is enabled)
pub fn debug(message: &str, debug_enabled: bool) {
    get_logger().debug(message, debug_enabled);
}

/// Show a shell command being executed
pub fn shell(command: &str) {
    get_logger().shell(command);
}

/// Show progress with spinner and counter
pub fn progress(message: &str, current: usize, total: usize) {
    get_logger().progress(message, current, total);
}

/// Show the final completion message and clear any updating lines
pub fn finish(message: &str) {
    get_logger().finish(message);
}

/// Show a final message without timing information
pub fn finish_line(message: &str) {
    get_logger().finish_line(message);
}
