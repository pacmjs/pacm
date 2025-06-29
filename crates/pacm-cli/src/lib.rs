use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pacm")]
#[command(version = "0.1.0")]
#[command(author = "Jonas Franke <me@binaryblazer.me>")]
#[command(propagate_version = true)]
#[command(about = "A super fast package manager for JavaScript/TypeScript", long_about = None)]
#[command(after_help = "For more information, visit <https://github.com/pacmjs/pacm>")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Installs all Dependencies from package.json
    #[command(aliases = ["i", "add"])]
    Install {
        /// List of packages to install (e.g. chalk@2.0.0)
        #[arg()]
        packages: Vec<String>,
    },
    /// Runs a script defined in package.json
    #[command(alias = "r")]
    Run {
        /// The name of the script (e.g. build, test, etc.)
        script: String,
    },
}

pub fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Install { packages } => {
            if packages.is_empty() {
                pacm_core::install_all_deps(".")?;
            } else {
                for pkg in packages {
                    let (name, version_range) = match pkg.split_once('@') {
                        Some((n, v)) => (n, v),
                        None => (pkg.as_str(), "latest"),
                    };
                    pacm_core::install_single_dep(".", name, version_range)?;
                }
            }
        }
        Commands::Run { script } => {
            pacm_runtime::run_script(".", script)?;
        }
    }

    Ok(())
}
