use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::install::CachedPackage;
use pacm_error::Result;
use pacm_logger;
use pacm_resolver::ResolvedPackage;
use pacm_store::link_package;

pub struct CacheLinker;

impl CacheLinker {
    pub fn verify_and_fix_deps(
        cached_packages: &[CachedPackage],
        all_stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        if cached_packages.is_empty() {
            return Ok(());
        }

        pacm_logger::status("Verifying cached package dependencies...");

        for cached_pkg in cached_packages {
            let package_node_modules = cached_pkg.store_path.join("package").join("node_modules");

            let cached_key = format!("{}@{}", cached_pkg.name, cached_pkg.version);
            if let Some((resolved_pkg, _)) = all_stored_packages.get(&cached_key) {
                pacm_logger::debug(
                    &format!(
                        "Checking dependencies for cached package {}@{}",
                        cached_pkg.name, cached_pkg.version
                    ),
                    debug,
                );

                let mut needs_linking = false;

                for (dep_name, _dep_range) in &resolved_pkg.dependencies {
                    let dep_link_path = get_dep_link_path(&package_node_modules, dep_name);

                    if !dep_link_path.exists() || !is_valid_package_link(&dep_link_path, debug) {
                        pacm_logger::debug(
                            &format!(
                                "Missing or invalid dependency link: {} for {}",
                                dep_name, cached_pkg.name
                            ),
                            debug,
                        );
                        needs_linking = true;
                        break;
                    }
                }

                if needs_linking {
                    Self::relink_deps(cached_pkg, resolved_pkg, all_stored_packages, debug)?;
                }
            }
        }

        Ok(())
    }

    fn relink_deps(
        cached_pkg: &CachedPackage,
        resolved_pkg: &ResolvedPackage,
        all_stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        let package_node_modules = cached_pkg.store_path.join("package").join("node_modules");

        pacm_logger::debug(
            &format!(
                "Relinking dependencies for cached package {}@{}",
                cached_pkg.name, cached_pkg.version
            ),
            debug,
        );

        if let Err(e) = std::fs::create_dir_all(&package_node_modules) {
            pacm_logger::debug(
                &format!(
                    "Failed to create node_modules for {}: {}",
                    cached_pkg.name, e
                ),
                debug,
            );
            return Ok(());
        }

        for (dep_name, _dep_range) in &resolved_pkg.dependencies {
            if let Some((_, dep_store_path)) = all_stored_packages
                .iter()
                .find(|(key, _)| key.starts_with(&format!("{}@", dep_name)))
                .map(|(_, (_, store_path))| ((), store_path))
            {
                if let Err(e) = link_package(&package_node_modules, dep_name, dep_store_path) {
                    pacm_logger::debug(
                        &format!(
                            "Failed to relink dependency {} for cached package {}: {}",
                            dep_name, cached_pkg.name, e
                        ),
                        debug,
                    );
                } else {
                    pacm_logger::debug(
                        &format!(
                            "Successfully linked dependency {} for {}",
                            dep_name, cached_pkg.name
                        ),
                        debug,
                    );
                }
            }
        }

        Ok(())
    }
}

fn get_dep_link_path(package_node_modules: &Path, dep_name: &str) -> PathBuf {
    if dep_name.starts_with('@') {
        if let Some(slash_pos) = dep_name.find('/') {
            let scope = &dep_name[..slash_pos];
            let name = &dep_name[slash_pos + 1..];
            package_node_modules.join(scope).join(name)
        } else {
            package_node_modules.join(dep_name)
        }
    } else {
        package_node_modules.join(dep_name)
    }
}

fn is_valid_package_link(link_path: &Path, _debug: bool) -> bool {
    if link_path.is_symlink() {
        if let Ok(target) = link_path.read_link() {
            let package_json = target.join("package.json");
            package_json.exists()
        } else {
            false
        }
    } else if link_path.is_dir() {
        link_path.join("package.json").exists()
    } else {
        false
    }
}
