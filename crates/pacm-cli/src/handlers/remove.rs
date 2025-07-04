use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;

pub struct RemoveHandler;

impl RemoveHandler {
    pub fn handle_remove_packages(packages: &[String], dev: bool, debug: bool) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        Self::print_remove_header(packages);

        pacm_core::remove_multiple_deps(".", packages, dev, debug)?;

        Ok(())
    }

    fn print_remove_header(packages: &[String]) {
        if packages.len() == 1 {
            println!(
                "{} {} {}",
                "pacm".bright_cyan().bold(),
                "remove".bright_white(),
                packages[0].bright_white()
            );
        } else {
            println!(
                "{} {} {}",
                "pacm".bright_cyan().bold(),
                "remove".bright_white(),
                packages.join(" ").bright_white()
            );
        }
        println!();
    }
}
