use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;
use pacm_logger;

pub struct CleanHandler;

impl CleanHandler {
    pub fn handle_clean(cache: bool, modules: bool, yes: bool, debug: bool) -> Result<()> {
        if !cache && !modules {
            pacm_logger::error("Please specify what to clean: --cache, --modules, or both");
            return Ok(());
        }

        Self::print_clean_header();

        if cache {
            Self::clean_cache(yes, debug)?;
        }

        if modules {
            Self::clean_node_modules(yes, debug)?;
        }

        Ok(())
    }

    fn clean_cache(yes: bool, debug: bool) -> Result<()> {
        if !yes {
            println!();
            println!(
                "{} {}",
                "⚠️ ".bright_yellow(),
                "CACHE CLEANING WARNING".bright_yellow().bold()
            );
            println!();
            println!(
                "{}",
                "This will remove ALL cached packages from the global store.".bright_red()
            );
            println!(
                "{}",
                "You will need to re-download packages for future installations.".bright_red()
            );
            println!();

            // In a real implementation, you would prompt for confirmation
            // For now, we'll just proceed with a warning
            pacm_logger::info("Proceeding with cache cleaning...");
        }

        pacm_core::clean_cache(debug)
    }

    fn clean_node_modules(yes: bool, debug: bool) -> Result<()> {
        if !yes {
            println!();
            println!(
                "{} {}",
                "⚠️ ".bright_yellow(),
                "NODE_MODULES CLEANING WARNING".bright_yellow().bold()
            );
            println!();
            println!(
                "{}",
                "This will remove the local node_modules directory.".bright_red()
            );
            println!(
                "{}",
                "You will need to run 'pacm install' to restore dependencies.".bright_red()
            );
            println!();

            // In a real implementation, you would prompt for confirmation
            // For now, we'll just proceed with a warning
            pacm_logger::info("Proceeding with node_modules cleaning...");
        }

        pacm_core::clean_node_modules(".", debug)
    }

    fn print_clean_header() {
        println!("{} {}", "pacm".bright_cyan().bold(), "clean".bright_white());
        println!();
    }
}
