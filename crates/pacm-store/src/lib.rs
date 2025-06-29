use std::{
    fs, io,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

/// Path to the central store (e.g., ~/.pacm/store)
pub fn get_store_path() -> PathBuf {
    dirs::home_dir().unwrap().join(".pacm").join("store")
}

/// Saves the package in the store and returns the store path
pub fn store_package(
    package_name: &str,
    version: &str,
    tarball_bytes: &[u8],
) -> io::Result<PathBuf> {
    // Calculate hash based on package name, version, and tarball content
    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(tarball_bytes);
        format!("{:x}", hasher.finalize())
    };

    let path = get_store_path()
        .join("npm")
        .join(format!("{package_name}@{version}-{hash}"));

    if path.exists() {
        return Ok(path);
    }

    // Extract tarball
    let temp_dir = tempfile::tempdir()?;
    let tar = flate2::read::GzDecoder::new(tarball_bytes);
    let mut archive = tar::Archive::new(tar);
    archive.unpack(temp_dir.path())?;

    fs::create_dir_all(&path)?;
    fs_extra::dir::copy(
        temp_dir.path(),
        &path,
        &fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true),
    )
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(path)
}

/// Creates a symlink in the project to the global store
pub fn link_package(
    project_node_modules: &Path,
    package_name: &str,
    store_path: &Path,
) -> io::Result<()> {
    let dest = project_node_modules.join(package_name);

    fs::create_dir_all(project_node_modules)?;

    if dest.exists() {
        fs::remove_file(&dest)?;
    }

    let updated_store_path = store_path
        .canonicalize()
        .map(|p| p.join("package"))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    #[cfg(target_family = "unix")]
    std::os::unix::fs::symlink(updated_store_path, &dest)?;

    #[cfg(target_family = "windows")]
    std::os::windows::fs::symlink_dir(updated_store_path, &dest)?;

    Ok(())
}
