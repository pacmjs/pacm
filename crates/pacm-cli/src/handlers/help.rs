use anyhow::Result;
use clap::CommandFactory;
use owo_colors::OwoColorize;

use crate::commands::Cli;
use pacm_constants::{BIN_NAME, COMMANDS, DESCRIPTION, EXAMPLES, REPOSITORY_URL, VERSION};

pub struct HelpHandler;

impl HelpHandler {
    pub fn handle_help(command: Option<&str>) -> Result<()> {
        match command {
            Some(cmd) => Self::show_command_help(cmd),
            None => Self::show_general_help(),
        }
    }

    fn show_general_help() -> Result<()> {
        Self::show_custom_help();
        Ok(())
    }

    fn show_command_help(command: &str) -> Result<()> {
        let mut cmd = Cli::command();

        if let Some(subcommand) = cmd.find_subcommand_mut(command) {
            subcommand.print_help()?;
        } else {
            println!(
                "{}: Unknown command '{}'",
                "Error".bright_red().bold(),
                command
            );
            println!();
            Self::show_custom_help();
        }

        println!();
        Ok(())
    }

    fn show_custom_help() {
        // Header
        println!("{}", DESCRIPTION.bright_white().bold());
        println!(
            "{} {}",
            "Version:".bright_white().bold(),
            VERSION.bright_black().bold()
        );
        println!();

        // Usage
        println!("{}", "Usage:".bright_magenta().bold());
        println!(
            "  {} {} {} {}",
            BIN_NAME.bright_cyan().bold(),
            "<COMMAND>".bright_white(),
            "<OPTIONS>".bright_black().bold(),
            "[ARGS]".bright_black().bold()
        );
        println!();

        // Commands
        println!("{}", "Commands:".bright_magenta().bold());
        let commands = COMMANDS
            .iter()
            .map(|(cmd, desc, aliases)| (cmd.to_string(), desc.to_string(), aliases.to_vec()))
            .collect::<Vec<_>>();

        let max_cmd_width = commands
            .iter()
            .map(|(cmd, _, aliases)| {
                let alias_str = if aliases.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", aliases.join(", "))
                };
                cmd.len() + alias_str.len()
            })
            .max()
            .unwrap_or(0);

        for (cmd, desc, aliases) in commands {
            let alias_str = if aliases.is_empty() {
                String::new()
            } else {
                format!(" [{}]", aliases.join(", "))
            };
            let plain_cmd = format!("{}{}", cmd, alias_str);
            let colored_cmd = format!(
                "{}{}",
                cmd.bright_cyan().bold(),
                alias_str.bright_black().bold()
            );
            println!(
                "  {:width$}  # {}",
                colored_cmd,
                desc.bright_black().bold(),
                width = max_cmd_width + (colored_cmd.len() - plain_cmd.len())
            );
        }

        println!();

        // Options
        println!("{}", "Options:".bright_magenta().bold());
        let option_cmd = "-V, --version";
        let colored_option_str = format!("{}", option_cmd.bright_cyan().bold());
        println!(
            "  {}           # {}",
            colored_option_str,
            "Print version".bright_black().bold(),
        );
        println!();

        Self::show_additional_info();
    }

    fn show_additional_info() {
        println!("{}", "Examples:".bright_magenta().bold());

        let examples = EXAMPLES
            .iter()
            .map(|(cmd, desc)| (cmd.to_string(), desc.to_string()))
            .collect::<Vec<_>>();

        let max_example_width = examples.iter().map(|(cmd, _)| cmd.len()).max().unwrap_or(0);

        for (cmd, desc) in examples {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            let formatted_cmd = if parts.len() > 1 {
                let mut formatted_parts = vec![parts[0].bright_cyan().bold().to_string()];

                for part in &parts[1..] {
                    if part.starts_with('-') {
                        formatted_parts.push(part.bright_black().bold().to_string());
                    } else {
                        formatted_parts.push(part.bright_white().to_string());
                    }
                }

                formatted_parts.join(" ")
            } else {
                parts[0].bright_cyan().bold().to_string()
            };

            let visual_width_diff = formatted_cmd.len() - cmd.len();

            println!(
                "  {:width$}  # {}",
                formatted_cmd,
                desc.bright_black().bold(),
                width = max_example_width + visual_width_diff
            );
        }

        println!();
        println!();
        println!(
            "{}",
            "For more information about a specific command, use:".bright_magenta()
        );

        let help_cmd = "pacm help <command>";
        let formatted_help_cmd = format!(
            "{} {} {}",
            "pacm".bright_cyan().bold(),
            "help".bright_white(),
            "<command>".bright_black().bold()
        );
        let help_desc = "Show help for specific command";
        let visual_width_diff = formatted_help_cmd.len() - help_cmd.len();

        println!(
            "  {:width$}  # {}",
            formatted_help_cmd,
            help_desc.bright_black().bold(),
            width = max_example_width + visual_width_diff
        );
        println!();
        println!();
        println!(
            "Visit {} for more information",
            REPOSITORY_URL.bright_cyan().underline()
        );
    }
}
