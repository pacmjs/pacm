use std::path::{Path, PathBuf};

pub struct PathResolver;

impl PathResolver {
    #[must_use]
    pub fn resolve_store_package_path(
        store_base: &Path,
        package_name: &str,
        version: &str,
        _hash: &str, // Hash no longer used in path structure
    ) -> PathBuf {
        let safe_package_name = Self::sanitize_package_name(package_name);
        store_base
            .join("npm")
            .join(&safe_package_name)
            .join(version)
    }

    #[must_use]
    pub fn get_package_path(store_base: &Path, package_name: &str, version: &str) -> PathBuf {
        let safe_package_name = Self::sanitize_package_name(package_name);
        store_base
            .join("npm")
            .join(&safe_package_name)
            .join(version)
    }

    #[must_use]
    pub fn get_package_base_path(store_base: &Path, package_name: &str) -> PathBuf {
        let safe_package_name = Self::sanitize_package_name(package_name);
        store_base.join("npm").join(&safe_package_name)
    }

    #[must_use]
    pub fn sanitize_package_name(package_name: &str) -> String {
        if package_name.starts_with('@') {
            package_name.replace('@', "_at_").replace('/', "_slash_")
        } else {
            package_name.to_string()
        }
    }

    #[must_use]
    pub fn get_package_directory(store_path: &Path) -> PathBuf {
        store_path.join("package")
    }

    #[must_use]
    pub fn get_package_node_modules(store_path: &Path) -> PathBuf {
        Self::get_package_directory(store_path).join("node_modules")
    }
}
