use std::path::PathBuf;

use pacm_logger;
use pacm_project::read_package_json;
use pacm_error::{PackageManagerError, Result};

pub struct ListManager;

impl ListManager {
    pub fn list_dependencies(
        &self,
        project_dir: &str,
        tree: bool,
        _depth: Option<u32>,
    ) -> Result<()> {
        let path = PathBuf::from(project_dir);
        let pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        if tree {
            self.show_dependency_tree()
        } else {
            self.show_flat_list(&pkg)
        }
    }

    fn show_dependency_tree(&self) -> Result<()> {
        pacm_logger::info("Dependency tree:");
        pacm_logger::info("Tree view not yet implemented");
        Ok(())
    }

    fn show_flat_list(&self, pkg: &pacm_project::PackageJson) -> Result<()> {
        if let Some(deps) = &pkg.dependencies {
            if !deps.is_empty() {
                pacm_logger::info("Dependencies:");
                for (name, version) in deps {
                    println!("  {} {}", name, version);
                }
            }
        }

        if let Some(dev_deps) = &pkg.dev_dependencies {
            if !dev_deps.is_empty() {
                pacm_logger::info("DevDependencies:");
                for (name, version) in dev_deps {
                    println!("  {} {}", name, version);
                }
            }
        }

        if let Some(peer_deps) = &pkg.peer_dependencies {
            if !peer_deps.is_empty() {
                pacm_logger::info("PeerDependencies:");
                for (name, version) in peer_deps {
                    println!("  {} {}", name, version);
                }
            }
        }

        if let Some(opt_deps) = &pkg.optional_dependencies {
            if !opt_deps.is_empty() {
                pacm_logger::info("OptionalDependencies:");
                for (name, version) in opt_deps {
                    println!("  {} {}", name, version);
                }
            }
        }

        Ok(())
    }
}
