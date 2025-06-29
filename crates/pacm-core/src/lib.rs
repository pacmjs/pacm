use std::process::Command;
use std::{collections::HashSet, path::PathBuf};

use pacm_lock::{LockDependency, PacmLock};
use pacm_project::read_package_json;
use pacm_resolver::{ResolvedPackage, resolve_full_tree};
use pacm_store::{link_package, store_package};

pub fn install_all_deps(project_dir: &str) -> anyhow::Result<()> {
    let now = std::time::Instant::now();

    let path = PathBuf::from(project_dir);
    let pkg = read_package_json(&path)?;

    let mut deps = vec![];

    if let Some(dependencies) = pkg.dependencies {
        deps.extend(dependencies.into_iter());
    }

    if let Some(dev_dependencies) = pkg.dev_dependencies {
        deps.extend(dev_dependencies.into_iter());
    }

    // Load or create the lockfile
    let lock_path = path.join("pacm.lock");
    let mut lockfile = PacmLock::load(&lock_path)?;

    // Collect all dependencies
    let mut seen = HashSet::new();
    let mut all_packages = Vec::<ResolvedPackage>::new();

    for (name, version_range) in deps {
        let pkgs = resolve_full_tree(&name, &version_range, &mut seen)?;
        all_packages.extend(pkgs);
    }

    // Installation + Linking
    let node_modules_path = path.join("node_modules");
    for pkg in &all_packages {
        println!("📦 Installing {}@{}...", pkg.name, pkg.version);

        let tarball_bytes = reqwest::blocking::get(&pkg.resolved)?.bytes()?;

        let store_path = store_package(&pkg.name, &pkg.version, &tarball_bytes)?;
        link_package(&node_modules_path, &pkg.name, &store_path)?;

        lockfile.update_dep(
            &pkg.name,
            LockDependency {
                version: pkg.version.clone(),
                resolved: pkg.resolved.clone(),
                integrity: pkg.integrity.clone(),
            },
        );

        // Check for postinstall script
        let package_json_path = store_path.join("package.json");
        if package_json_path.exists() {
            let file = std::fs::File::open(&package_json_path)?;
            let pkg_data: serde_json::Value = serde_json::from_reader(file)?;
            if let Some(script) = pkg_data
                .get("scripts")
                .and_then(|s| s.get("postinstall"))
                .and_then(|s| s.as_str())
            {
                println!("🚀 Running postinstall for {}...", pkg.name);

                // Run the postinstall script
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(script)
                    .current_dir(&store_path)
                    .status()?;

                if !status.success() {
                    eprintln!("⚠️ Postinstall script for {} failed.", pkg.name);
                }
            }
        }
    }

    lockfile.save(&lock_path)?;
    println!("✅ All dependencies installed successfully!");
    println!("⌛ Installation took: {:.2?}", now.elapsed());
    Ok(())
}

pub fn install_single_dep(
    project_dir: &str,
    name: &str,
    version_range: &str,
) -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    let path = PathBuf::from(project_dir);

    // Load or create the lockfile
    let lock_path = path.join("pacm.lock");
    let mut lockfile = PacmLock::load(&lock_path)?;

    // Collect the single dependency
    let mut seen = HashSet::new();
    let pkgs = resolve_full_tree(name, version_range, &mut seen)?;

    // Installation + Linking
    let node_modules_path = path.join("node_modules");
    for pkg in &pkgs {
        println!("📦 Installing {}@{}...", pkg.name, pkg.version);

        let tarball_bytes = reqwest::blocking::get(&pkg.resolved)?.bytes()?;

        let store_path = store_package(&pkg.name, &pkg.version, &tarball_bytes)?;
        link_package(&node_modules_path, &pkg.name, &store_path)?;

        lockfile.update_dep(
            &pkg.name,
            LockDependency {
                version: pkg.version.clone(),
                resolved: pkg.resolved.clone(),
                integrity: pkg.integrity.clone(),
            },
        );

        // Check for postinstall script
        let package_json_path = store_path.join("package.json");
        if package_json_path.exists() {
            let file = std::fs::File::open(&package_json_path)?;
            let pkg_data: serde_json::Value = serde_json::from_reader(file)?;
            if let Some(script) = pkg_data
                .get("scripts")
                .and_then(|s| s.get("postinstall"))
                .and_then(|s| s.as_str())
            {
                println!("🚀 Running postinstall for {}...", pkg.name);

                let status = Command::new("sh")
                    .arg("-c")
                    .arg(script)
                    .current_dir(&store_path)
                    .status()?;

                if !status.success() {
                    eprintln!("⚠️ Postinstall script for {} failed.", pkg.name);
                }
            }
        }
    }

    lockfile.save(&lock_path)?;
    println!("✅ {} installed successfully!", name);
    println!("⌛ Installation took: {:.2?}", now.elapsed());
    Ok(())
}
