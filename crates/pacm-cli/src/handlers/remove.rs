use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;

pub struct RemoveHandler;

impl RemoveHandler {
    pub fn handle_remove_packages(
        packages: &[String],
        dev: bool,
        direct_only: bool,
        dry_run: bool,
        debug: bool,
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        Self::print_remove_header(packages, direct_only, dry_run);

        if dry_run {
            pacm_core::remove_multiple_deps_dry_run(".", packages, dev, direct_only, debug)?;
        } else if direct_only {
            pacm_core::remove_multiple_deps_direct_only(".", packages, dev, debug)?;
        } else {
            pacm_core::remove_multiple_deps(".", packages, dev, debug)?;
        }

        Ok(())
    }

    fn print_remove_header(packages: &[String], direct_only: bool, dry_run: bool) {
        let mode_text = if dry_run {
            " (dry run)".dimmed()
        } else if direct_only {
            " (direct only)".dimmed()
        } else {
            "".dimmed()
        };

        if packages.len() == 1 {
            println!(
                "{} {} {}{}",
                "pacm".bright_cyan().bold(),
                "remove".bright_white(),
                packages[0].bright_white(),
                mode_text
            );
        } else {
            println!(
                "{} {} {}{}",
                "pacm".bright_cyan().bold(),
                "remove".bright_white(),
                packages.join(" ").bright_white(),
                mode_text
            );
        }
        println!();
    }
}
