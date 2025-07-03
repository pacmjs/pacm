pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = "A super fast package manager for JavaScript/TypeScript";
pub const REPOSITORY_URL: &str = "https://github.com/pacmjs/pacm";
pub const BIN_NAME: &str = "pacm";
pub const COMMANDS: &[(&str, &str, &[&str])] = &[
    (
        "install",
        "Installs all Dependencies from package.json",
        &["i", "add"],
    ),
    ("init", "Initializes a new package.json file", &["new"]),
    ("run", "Runs a script defined in package.json", &["r"]),
    (
        "start",
        "Starts the application (runs start script or main entry point)",
        &[],
    ),
    ("remove", "Removes packages", &["rm", "uninstall"]),
    (
        "update",
        "Updates packages to their latest versions",
        &["up", "upgrade"],
    ),
    ("list", "Lists installed packages", &["ls"]),
    (
        "clean",
        "Cleans package cache and optionally local node_modules",
        &[],
    ),
    (
        "help",
        "Shows help information for pacm or a specific command",
        &[],
    ),
];
pub const EXAMPLES: &[(&str, &str)] = &[
    ("pacm install", "Install all dependencies"),
    ("pacm install axios", "Install a package"),
    ("pacm install typescript --dev", "Install dev dependency"),
    ("pacm update", "Update all packages"),
    ("pacm remove axios", "Remove a package"),
    ("pacm list", "List dependencies"),
    ("pacm init", "Initialize new project"),
    ("pacm clean --cache", "Clean package cache"),
];

pub const USER_AGENT: &str = "pacm/0.1.0";
pub const MAX_ATTEMPTS: u32 = 4;
pub const POPULAR_PACKAGES: &[&str] = &[
    "react",
    "vue",
    "angular",
    "express",
    "lodash",
    "axios",
    "typescript",
    "webpack",
    "babel-core",
    "eslint",
    "prettier",
    "jest",
    "mocha",
    "chai",
    "moment",
    "dotenv",
    "cors",
    "helmet",
    "bcrypt",
    "jsonwebtoken",
];
