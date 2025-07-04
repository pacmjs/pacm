use std::collections::HashMap;
use std::path::PathBuf;

use pacm_error::Result;
use pacm_logger;
use pacm_resolver::ResolvedPackage;

pub struct StoreLinker;

impl StoreLinker {
    pub fn link_deps_to_store(
        _stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug(
                "Skipping store dependency linking - using flat node_modules structure",
                debug,
            );
        }
        Ok(())
    }
}
