// Re-export public types and error handling
pub mod download;
pub mod error;
pub mod init;
pub mod install;
pub mod linker;
pub mod list;
pub mod remove;
pub mod update;

pub use error::{PackageManagerError, Result};
pub use init::InitManager;
pub use install::InstallManager;
pub use list::ListManager;
pub use remove::RemoveManager;
pub use update::UpdateManager;

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

pub fn install_all_deps(project_dir: &str, debug: bool) -> anyhow::Result<()> {
    let manager = InstallManager::new();
    manager
        .install_all_dependencies(project_dir, debug)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn install_single_dep(
    project_dir: &str,
    name: &str,
    version_range: &str,
    debug: bool,
) -> anyhow::Result<()> {
    let manager = InstallManager::new();
    manager
        .install_single_dependency(
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

pub fn install_single_dep_enhanced(
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
        .install_single_dependency(
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

pub fn remove_dependency(
    project_dir: &str,
    name: &str,
    dev_only: bool,
    debug: bool,
) -> anyhow::Result<()> {
    let manager = RemoveManager;
    manager
        .remove_dependency(project_dir, name, dev_only, debug)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn update_dependencies(
    project_dir: &str,
    packages: &[String],
    debug: bool,
) -> anyhow::Result<()> {
    let manager = UpdateManager::new();
    manager
        .update_dependencies(project_dir, packages, debug)
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn list_dependencies(project_dir: &str, tree: bool, depth: Option<u32>) -> anyhow::Result<()> {
    let manager = ListManager;
    manager
        .list_dependencies(project_dir, tree, depth)
        .map_err(|e| anyhow::anyhow!(e))
}
