use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_runtime;

pub struct RunHandler;

impl RunHandler {
    pub fn handle_run_script(script: &str) -> Result<()> {
        Self::print_run_header(script);
        pacm_runtime::run_script(".", script)
    }

    fn print_run_header(script: &str) {
        println!(
            "{} {} {}",
            "pacm".bright_cyan().bold(),
            "run".bright_white(),
            script.bright_white()
        );
        println!();
    }
}
