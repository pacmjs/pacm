use std::path::PathBuf;
use std::process::Command;

use pacm_logger;
use pacm_project::read_package_json;

pub fn run_script(project_dir: &str, script_name: &str) -> anyhow::Result<()> {
    let path = PathBuf::from(project_dir);
    let pkg = read_package_json(&path)?;

    if let Some(scripts) = pkg.scripts {
        if let Some(script) = scripts.get(script_name) {
            pacm_logger::info(&format!("Running script '{}'...", script_name));

            // Use different command based on OS
            let status = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(["/C", script])
                    .current_dir(&path)
                    .status()?
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(script)
                    .current_dir(&path)
                    .status()?
            };

            if status.success() {
                pacm_logger::success(&format!("Script '{}' executed successfully!", script_name));
            } else {
                pacm_logger::error(&format!(
                    "Script '{}' failed with exit code: {}",
                    script_name,
                    status.code().unwrap_or(-1)
                ));
            }
        } else {
            pacm_logger::error(&format!(
                "Script '{}' not found in package.json",
                script_name
            ));
        }
    } else {
        pacm_logger::error("No scripts defined in package.json");
    }

    Ok(())
}
