use anyhow::Result;

use pacm_runtime;

pub struct StartHandler;

impl StartHandler {
    pub fn handle_start() -> Result<()> {
        pacm_runtime::start_application(".")
    }
}
