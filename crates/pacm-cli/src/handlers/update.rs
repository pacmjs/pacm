use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;

pub struct UpdateHandler;

impl UpdateHandler {
    pub fn handle_update_packages(packages: &[String], debug: bool) -> Result<()> {
        Self::print_update_header();
        pacm_core::update_deps(".", packages, debug)
    }

    fn print_update_header() {
        println!(
            "{} {}",
            "pacm".bright_cyan().bold(),
            "update".bright_white()
        );
        println!();
    }
}
