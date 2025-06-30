use anyhow::Result;

use pacm_core;

pub struct ListHandler;

impl ListHandler {
    pub fn handle_list_dependencies(tree: bool, depth: Option<u32>) -> Result<()> {
        pacm_core::list_deps(".", tree, depth)
    }
}
