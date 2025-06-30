use std::path::{Path, PathBuf};

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir_exists(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Get the node_modules directory for a project
pub fn get_node_modules_path(project_dir: &Path) -> PathBuf {
    project_dir.join("node_modules")
}

/// Get the package.json path for a project
pub fn get_package_json_path(project_dir: &Path) -> PathBuf {
    project_dir.join("package.json")
}

/// Get the lock file path for a project
pub fn get_lock_file_path(project_dir: &Path) -> PathBuf {
    project_dir.join("pacm.lock")
}

/// Handle scoped package names in file paths
pub fn get_scoped_package_path(base_path: &Path, package_name: &str) -> PathBuf {
    if package_name.starts_with('@') {
        if let Some(slash_pos) = package_name.find('/') {
            let scope = &package_name[..slash_pos]; // @types
            let name = &package_name[slash_pos + 1..]; // node
            let scope_dir = base_path.join(scope);
            scope_dir.join(name)
        } else {
            base_path.join(package_name)
        }
    } else {
        base_path.join(package_name)
    }
}
