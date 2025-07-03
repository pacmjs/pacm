pub mod clean;
pub mod download;
pub mod init;
pub mod install;
pub mod linker;
pub mod list;
pub mod remove;
pub mod update;

pub use clean::CleanManager;
pub use init::InitManager;
pub use install::InstallManager;
pub use list::ListManager;
pub use remove::RemoveManager;
pub use update::UpdateManager;

use pacm_error::Result;
use pacm_project::DependencyType;

pub fn init_project(
    project_dir: &str,
    name: &str,
    description: Option<&str>,
    version: Option<&str>,
    license: Option<&str>,
) -> Result<()> {
    let manager = InitManager::new();
    manager.init_project(project_dir, name, description, version, license)
}

pub fn init_interactive(project_dir: &str, yes: bool) -> anyhow::Result<()> {
    let manager = InitManager::new();
    manager
        .init_interactive(project_dir, yes)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn install_all(project_dir: &str, debug: bool) -> anyhow::Result<()> {
    let manager = InstallManager::new();
    manager
        .install_all(project_dir, debug)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn install_single(
    project_dir: &str,
    name: &str,
    version_range: &str,
    debug: bool,
) -> anyhow::Result<()> {
    let manager = InstallManager::new();
    manager
        .install_single(
            project_dir,
            name,
            version_range,
            DependencyType::Dependencies,
            false, // save_exact
            false, // no_save
            false, // force
            debug,
        )
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn install_enhanced(
    project_dir: &str,
    name: &str,
    version_range: &str,
    dep_type: DependencyType,
    save_exact: bool,
    no_save: bool,
    force: bool,
    debug: bool,
) -> anyhow::Result<()> {
    let manager = InstallManager::new();
    manager
        .install_single(
            project_dir,
            name,
            version_range,
            dep_type,
            save_exact,
            no_save,
            force,
            debug,
        )
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn install_multiple(
    project_dir: &str,
    packages: &[(String, String)], // (name, version_range) pairs
    dep_type: DependencyType,
    save_exact: bool,
    no_save: bool,
    force: bool,
    debug: bool,
) -> anyhow::Result<()> {
    let manager = InstallManager::new();
    manager
        .install_multiple(
            project_dir,
            packages,
            dep_type,
            save_exact,
            no_save,
            force,
            debug,
        )
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn remove_dep(
    project_dir: &str,
    name: &str,
    dev_only: bool,
    debug: bool,
) -> anyhow::Result<()> {
    let manager = RemoveManager;
    manager
        .remove_dep(project_dir, name, dev_only, debug)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn update_deps(project_dir: &str, packages: &[String], debug: bool) -> anyhow::Result<()> {
    let manager = UpdateManager::new();
    manager
        .update_deps(project_dir, packages, debug)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn list_deps(project_dir: &str, tree: bool, depth: Option<u32>) -> anyhow::Result<()> {
    let manager = ListManager;
    manager
        .list_deps(project_dir, tree, depth)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn clean_cache(debug: bool) -> anyhow::Result<()> {
    let manager = CleanManager::new();
    manager.clean_cache(debug).map_err(|e| anyhow::anyhow!(e))
}

pub fn clean_node_modules(project_dir: &str, debug: bool) -> anyhow::Result<()> {
    let manager = CleanManager::new();
    manager
        .clean_node_modules(project_dir, debug)
        .map_err(|e| anyhow::anyhow!(e))
}
