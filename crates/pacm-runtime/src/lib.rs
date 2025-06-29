use std::path::PathBuf;
use std::process::Command;

use pacm_project::read_package_json;

pub fn run_script(project_dir: &str, script_name: &str) -> anyhow::Result<()> {
    let path = PathBuf::from(project_dir);
    let pkg = read_package_json(&path)?;

    if let Some(scripts) = pkg.scripts {
        if let Some(script) = scripts.get(script_name) {
            println!("🚀 Running script '{}'...", script_name);
            let status = Command::new("sh")
                .arg("-c")
                .arg(script)
                .current_dir(&path)
                .status()?;

            if status.success() {
                println!("✅ Script '{}' executed successfully!", script_name);
            } else {
                eprintln!(
                    "⚠️ Script '{}' failed with exit code: {}",
                    script_name,
                    status.code().unwrap_or(-1)
                );
            }
        } else {
            eprintln!("❌ Script '{}' not found in package.json", script_name);
        }
    } else {
        eprintln!("❌ No scripts defined in package.json");
    }

    Ok(())
}
