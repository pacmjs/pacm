use std::path::{Path, PathBuf};

pub struct PathResolver;

impl PathResolver {
    /// Resolve package path within store
    pub fn resolve_store_package_path(
        store_base: &Path,
        package_name: &str,
        version: &str,
        hash: &str,
    ) -> PathBuf {
        let safe_package_name = Self::sanitize_package_name(package_name);
        store_base
            .join("npm")
            .join(format!("{safe_package_name}@{version}-{hash}"))
    }

    /// Sanitize package name for filesystem usage
    pub fn sanitize_package_name(package_name: &str) -> String {
        if package_name.starts_with('@') {
            package_name.replace('@', "_at_").replace('/', "_slash_")
        } else {
            package_name.to_string()
        }
    }

    /// Get the canonical package directory within store
    pub fn get_package_directory(store_path: &Path) -> PathBuf {
        store_path.join("package")
    }

    /// Get node_modules directory for a package in store
    pub fn get_package_node_modules(store_path: &Path) -> PathBuf {
        Self::get_package_directory(store_path).join("node_modules")
    }
}
