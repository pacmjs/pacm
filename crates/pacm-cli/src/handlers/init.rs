use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;

pub struct InitHandler;

impl InitHandler {
    pub fn init_project(yes: bool) -> Result<()> {
        Self::print_init_header();
        pacm_core::init_interactive(".", yes)
    }

    fn print_init_header() {
        println!("{} {}", "pacm".bright_cyan().bold(), "init".bright_white());
        println!();
    }
}
