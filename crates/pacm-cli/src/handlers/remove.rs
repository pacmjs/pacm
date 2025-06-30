use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;

pub struct RemoveHandler;

impl RemoveHandler {
    pub fn handle_remove_packages(packages: &[String], dev: bool, debug: bool) -> Result<()> {
        for package in packages {
            Self::print_remove_header(package);
            pacm_core::remove_dep(".", package, dev, debug)?;
        }
        Ok(())
    }

    fn print_remove_header(package: &str) {
        println!(
            "{} {} {}",
            "pacm".bright_cyan().bold(),
            "remove".bright_white(),
            package.bright_white()
        );
        println!();
    }
}
