use crate::package_json::{DependencyType, PackageJson};
use indexmap::IndexMap;

pub struct DependencyManager;

impl DependencyManager {
    pub fn add_dep(
        package_json: &mut PackageJson,
        name: &str,
        version: &str,
        dep_type: DependencyType,
        save_exact: bool,
    ) {
        let version_string = if save_exact {
            version.to_string()
        } else if version.starts_with('^') || version.starts_with('~') || version.contains('-') {
            version.to_string()
        } else {
            format!("^{version}")
        };

        Self::remove_dep(package_json, name);

        match dep_type {
            DependencyType::Dependencies => {
                package_json
                    .dependencies
                    .get_or_insert_with(IndexMap::new)
                    .insert(name.to_string(), version_string);
            }
            DependencyType::DevDependencies => {
                package_json
                    .dev_dependencies
                    .get_or_insert_with(IndexMap::new)
                    .insert(name.to_string(), version_string);
            }
            DependencyType::PeerDependencies => {
                package_json
                    .peer_dependencies
                    .get_or_insert_with(IndexMap::new)
                    .insert(name.to_string(), version_string);
            }
            DependencyType::OptionalDependencies => {
                package_json
                    .optional_dependencies
                    .get_or_insert_with(IndexMap::new)
                    .insert(name.to_string(), version_string);
            }
        }
    }

    pub fn remove_dep(package_json: &mut PackageJson, name: &str) {
        if let Some(deps) = &mut package_json.dependencies {
            deps.shift_remove(name);
        }
        if let Some(dev_deps) = &mut package_json.dev_dependencies {
            dev_deps.shift_remove(name);
        }
        if let Some(peer_deps) = &mut package_json.peer_dependencies {
            peer_deps.shift_remove(name);
        }
        if let Some(opt_deps) = &mut package_json.optional_dependencies {
            opt_deps.shift_remove(name);
        }
    }

    #[must_use]
    pub fn has_dep(package_json: &PackageJson, name: &str) -> Option<DependencyType> {
        if let Some(deps) = &package_json.dependencies {
            if deps.contains_key(name) {
                return Some(DependencyType::Dependencies);
            }
        }
        if let Some(dev_deps) = &package_json.dev_dependencies {
            if dev_deps.contains_key(name) {
                return Some(DependencyType::DevDependencies);
            }
        }
        if let Some(peer_deps) = &package_json.peer_dependencies {
            if peer_deps.contains_key(name) {
                return Some(DependencyType::PeerDependencies);
            }
        }
        if let Some(opt_deps) = &package_json.optional_dependencies {
            if opt_deps.contains_key(name) {
                return Some(DependencyType::OptionalDependencies);
            }
        }
        None
    }
}
