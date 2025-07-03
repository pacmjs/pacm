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
                    if potential_command == "help" {
                        pacm_logger::init_logger(false);
                        let help_command = if args.len() >= 3 {
                            Some(args[2].as_str())
                        } else {
                            None
                        };
                        HelpHandler::handle_help(help_command)
                    } else {
                        pacm_logger::init_logger(false);
                        RunHandler::handle_run_script(potential_command)
                    }
                } else {
                    let cli = Cli::parse();
                    pacm_logger::init_logger(false);
                    handle_known_command(&cli.command)
                }
            }
        }
    } else {
        pacm_logger::init_logger(false);
        HelpHandler::handle_help(None)
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
        Commands::Clean {
            cache,
            modules,
            yes,
            debug,
        } => CleanHandler::handle_clean(*cache, *modules, *yes, *debug),
        Commands::Help { command } => HelpHandler::handle_help(command.as_deref()),
    }
}
