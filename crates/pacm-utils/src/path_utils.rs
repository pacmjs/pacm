use std::path::{Path, PathBuf};

pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

#[must_use]
pub fn node_modules_path(project_dir: &Path) -> PathBuf {
    project_dir.join("node_modules")
}

#[must_use]
pub fn package_json_path(project_dir: &Path) -> PathBuf {
    project_dir.join("package.json")
}

#[must_use]
pub fn lock_file_path(project_dir: &Path) -> PathBuf {
    project_dir.join("pacm.lock")
}

#[must_use]
pub fn scoped_pkg_path(base_path: &Path, package_name: &str) -> PathBuf {
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
