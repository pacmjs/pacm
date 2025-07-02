pub mod dependency_manager;
pub mod io;
pub mod package_json;

pub use dependency_manager::DependencyManager;
pub use io::{read_package_json, write_package_json};
pub use package_json::{DependencyType, PackageJson};

impl PackageJson {
    pub fn add_dependency(
        &mut self,
        name: &str,
        version: &str,
        dep_type: DependencyType,
        save_exact: bool,
    ) {
        DependencyManager::add_dep(self, name, version, dep_type, save_exact);
    }

    pub fn remove_dependency(&mut self, name: &str) {
        DependencyManager::remove_dep(self, name);
    }

    #[must_use]
    pub fn has_dependency(&self, name: &str) -> Option<DependencyType> {
        DependencyManager::has_dep(self, name)
    }
}
