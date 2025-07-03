use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pacm")]
#[command(version = "0.1.0")]
#[command(author = "Jonas Franke <me@binaryblazer.me>")]
#[command(propagate_version = true)]
#[command(about = "A super fast package manager for JavaScript/TypeScript", long_about = None)]
#[command(disable_help_flag = true)]
#[command(disable_help_subcommand = true)]
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
        /// Install as devDependency
        #[arg(short = 'D', long = "dev", alias = "save-dev")]
        dev: bool,
        /// Install as optionalDependency
        #[arg(short = 'O', long = "optional")]
        optional: bool,
        /// Install as peerDependency
        #[arg(short = 'P', long = "peer")]
        peer: bool,
        /// Install globally
        #[arg(short = 'g', long = "global")]
        global: bool,
        /// Save exact version (no caret prefix)
        #[arg(short = 'E', long = "save-exact")]
        save_exact: bool,
        /// Don't save to package.json
        #[arg(long = "no-save")]
        no_save: bool,
        /// Force reinstall even if already installed
        #[arg(short = 'f', long = "force")]
        force: bool,
        /// Enable debug mode for verbose output
        #[arg(long)]
        debug: bool,
    },
    /// Initializes a new package.json file
    #[command(alias = "new")]
    Init {
        /// Skips interactive prompts
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Runs a script defined in package.json
    #[command(alias = "r")]
    Run {
        /// The name of the script (e.g. build, test, etc.)
        script: String,
    },
    /// Starts the application (runs start script or main entry point)
    Start,
    /// Removes packages
    #[command(aliases = ["rm", "uninstall"])]
    Remove {
        /// List of packages to remove
        #[arg(required = true)]
        packages: Vec<String>,
        /// Remove from devDependencies only
        #[arg(short = 'D', long = "dev")]
        dev: bool,
        /// Enable debug mode for verbose output
        #[arg(long)]
        debug: bool,
    },
    /// Updates packages to their latest versions
    #[command(aliases = ["up", "upgrade"])]
    Update {
        /// List of packages to update (if empty, updates all)
        #[arg()]
        packages: Vec<String>,
        /// Enable debug mode for verbose output
        #[arg(long)]
        debug: bool,
    },
    /// Lists installed packages
    #[command(alias = "ls")]
    List {
        /// Show dependency tree
        #[arg(long)]
        tree: bool,
        /// Show only top-level dependencies
        #[arg(long)]
        depth: Option<u32>,
    },
    /// Cleans package cache and optionally local node_modules
    Clean {
        /// Clear the global package cache/store
        #[arg(long = "cache")]
        cache: bool,
        /// Clear local node_modules directory
        #[arg(long = "modules")]
        modules: bool,
        /// Skip confirmation prompts
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Enable debug mode for verbose output
        #[arg(long)]
        debug: bool,
    },
    /// Shows help information for pacm or a specific command
    Help {
        /// The command to show help for (optional)
        #[arg()]
        command: Option<String>,
    },
}
