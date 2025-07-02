pub mod commands;
pub mod handlers;

use anyhow::Result;
use clap::Parser;
use std::env;

use commands::{Cli, Commands};
use handlers::*;

pub fn run_cli() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() >= 2 {
        let potential_command = &args[1];

        match Cli::try_parse() {
            Ok(cli) => {
                pacm_logger::init_logger(false);
                handle_known_command(&cli.command)
            }
            Err(_) => {
                if !potential_command.starts_with('-') && !potential_command.starts_with("--") {
                    pacm_logger::init_logger(false);
                    RunHandler::handle_run_script(potential_command)
                } else {
                    let cli = Cli::parse();
                    pacm_logger::init_logger(false);
                    handle_known_command(&cli.command)
                }
            }
        }
    } else {
        let cli = Cli::parse();
        pacm_logger::init_logger(false);
        handle_known_command(&cli.command)
    }
}

fn handle_known_command(command: &Commands) -> Result<()> {
    match command {
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
                InstallHandler::install_all(*debug)
            } else {
                InstallHandler::install_pkgs(
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
        Commands::Init { yes } => InitHandler::init_project(*yes),
        Commands::Run { script } => RunHandler::handle_run_script(script),
        Commands::Start => StartHandler::handle_start(),
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
