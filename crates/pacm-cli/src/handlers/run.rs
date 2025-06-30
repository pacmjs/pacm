use anyhow::Result;

use pacm_runtime;

pub struct RunHandler;

impl RunHandler {
    pub fn handle_run_script(script: &str) -> Result<()> {
        pacm_runtime::run_script(".", script)
    }
}
