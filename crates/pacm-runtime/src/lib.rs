use std::path::PathBuf;
use std::process::Command;

use pacm_logger;
use pacm_project::read_package_json;

pub fn run_script(project_dir: &str, script_name: &str) -> anyhow::Result<()> {
    let path = PathBuf::from(project_dir);
    let pkg = read_package_json(&path)?;

    if let Some(scripts) = pkg.scripts {
        if let Some(script) = scripts.get(script_name) {
            pacm_logger::shell(script);

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

pub fn start_application(project_dir: &str) -> anyhow::Result<()> {
    let path = PathBuf::from(project_dir);
    let pkg = read_package_json(&path)?;

    if let Some(scripts) = &pkg.scripts {
        if let Some(start_script) = scripts.get("start") {
            pacm_logger::shell(start_script);

            let status = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(["/C", start_script])
                    .current_dir(&path)
                    .status()?
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(start_script)
                    .current_dir(&path)
                    .status()?
            };

            if status.success() {
                pacm_logger::success("Start script executed successfully!");
            } else {
                pacm_logger::error(&format!(
                    "Start script failed with exit code: {}",
                    status.code().unwrap_or(-1)
                ));
            }
            return Ok(());
        }
    }

    if let Some(main) = &pkg.main {
        let main_path = path.join(main);
        if main_path.exists() {
            let command = format!("node {}", main);
            pacm_logger::shell(&command);

            let status = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(["/C", &command])
                    .current_dir(&path)
                    .status()?
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .current_dir(&path)
                    .status()?
            };

            if status.success() {
                pacm_logger::success("Application started successfully!");
            } else {
                pacm_logger::error(&format!(
                    "Application failed to start with exit code: {}",
                    status.code().unwrap_or(-1)
                ));
            }
        } else {
            pacm_logger::error(&format!("Main entry point '{}' does not exist", main));
        }
    } else {
        // Try common entry points if no main is specified
        let common_entries = ["index.js", "app.js", "server.js", "main.js"];
        let mut found = false;

        for entry in &common_entries {
            let entry_path = path.join(entry);
            if entry_path.exists() {
                pacm_logger::info(&format!("No main entry point specified, trying: {}", entry));

                let command = format!("node {}", entry);
                pacm_logger::shell(&command);

                let status = if cfg!(target_os = "windows") {
                    Command::new("cmd")
                        .args(["/C", &command])
                        .current_dir(&path)
                        .status()?
                } else {
                    Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .current_dir(&path)
                        .status()?
                };

                if status.success() {
                    pacm_logger::success("Application started successfully!");
                } else {
                    pacm_logger::error(&format!(
                        "Application failed to start with exit code: {}",
                        status.code().unwrap_or(-1)
                    ));
                }
                found = true;
                break;
            }
        }

        if !found {
            pacm_logger::error(
                "No start script found and no main entry point available. Please define a 'start' script in package.json or specify a 'main' field.",
            );
        }
    }

    Ok(())
}
