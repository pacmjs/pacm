pub mod commands;
pub mod handlers;

use anyhow::Result;
use clap::Parser;

use commands::{Cli, Commands};
use handlers::*;

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logger (quiet mode for now, could be a flag later)
    pacm_logger::init_logger(false);

    match &cli.command {
        Commands::Install {
            packages,
            dev,
            optional,
            peer,
            global,
            save_exact,
            no_save,
            force,
            debug,
        } => {
            if packages.is_empty() {
                InstallHandler::handle_install_all(*debug)
            } else {
                InstallHandler::handle_install_packages(
                    packages,
                    *dev,
                    *optional,
                    *peer,
                    *global,
                    *save_exact,
                    *no_save,
                    *force,
                    *debug,
                )
            }
        }
        Commands::Run { script } => RunHandler::handle_run_script(script),
        Commands::Remove {
            packages,
            dev,
            debug,
        } => RemoveHandler::handle_remove_packages(packages, *dev, *debug),
        Commands::Update { packages, debug } => {
            UpdateHandler::handle_update_packages(packages, *debug)
        }
        Commands::List { tree, depth } => ListHandler::handle_list_dependencies(*tree, *depth),
    }
}
