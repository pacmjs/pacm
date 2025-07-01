use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

use pacm_error::Result;
use pacm_logger;
use pacm_resolver::ResolvedPackage;
use pacm_store::link_package;

pub struct StoreLinker;

impl StoreLinker {
    pub fn link_deps_to_store(
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Setting up package dependencies...");

        stored_packages
            .par_iter()
            .for_each(|(_package_key, (pkg, store_path))| {
                if debug {
                    pacm_logger::debug(
                        &format!(
                            "Setting up dependencies for {}@{} in store",
                            pkg.name, pkg.version
                        ),
                        debug,
                    );
                }

                let package_node_modules = store_path.join("package").join("node_modules");

                pkg.dependencies
                    .par_iter()
                    .for_each(|(dep_name, _dep_range)| {
                        if let Some((_, dep_store_path)) = stored_packages
                            .iter()
                            .find(|(key, _)| key.starts_with(&format!("{}@", dep_name)))
                            .map(|(_, (_, store_path))| ((), store_path))
                        {
                            if let Err(e) =
                                link_package(&package_node_modules, dep_name, dep_store_path)
                            {
                                if debug {
                                    pacm_logger::debug(
                                        &format!(
                                            "Failed to link dependency {} for package {}: {}",
                                            dep_name, pkg.name, e
                                        ),
                                        debug,
                                    );
                                }
                            }
                        }
                    });
            });

        Ok(())
    }
}
