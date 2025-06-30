pub mod dependency_manager;
pub mod io;
pub mod package_json;

pub use dependency_manager::DependencyManager;
pub use io::{read_package_json, write_package_json};
pub use package_json::{DependencyType, PackageJson};

// Backward compatibility - delegate to methods
impl PackageJson {
    /// Add a dependency to the package.json
    pub fn add_dependency(
        &mut self,
        name: &str,
        version: &str,
        dep_type: DependencyType,
        save_exact: bool,
    ) {
        DependencyManager::add_dependency(self, name, version, dep_type, save_exact);
    }

    /// Remove a dependency from all dependency types
    pub fn remove_dependency(&mut self, name: &str) {
        DependencyManager::remove_dependency(self, name);
    }

    /// Check if a dependency exists in any dependency type
    pub fn has_dependency(&self, name: &str) -> Option<DependencyType> {
        DependencyManager::has_dependency(self, name)
    }
}
