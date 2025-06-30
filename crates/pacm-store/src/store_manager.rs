use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub struct StoreManager;

impl StoreManager {
    pub fn get_store_path() -> PathBuf {
        dirs::home_dir().unwrap().join(".pacm").join("store")
    }

    pub fn store_package(
        package_name: &str,
        version: &str,
        tarball_bytes: &[u8],
    ) -> io::Result<PathBuf> {
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(tarball_bytes);
            format!("{:x}", hasher.finalize())
        };

        let safe_package_name = Self::sanitize_package_name(package_name);
        let path = Self::get_store_path()
            .join("npm")
            .join(format!("{safe_package_name}@{version}-{hash}"));

        if path.exists() {
            return Ok(path);
        }

        Self::extract_and_store_package(&path, tarball_bytes)?;
        Ok(path)
    }

    fn sanitize_package_name(package_name: &str) -> String {
        if package_name.starts_with('@') {
            package_name.replace('@', "_at_").replace('/', "_slash_")
        } else {
            package_name.to_string()
        }
    }

    fn extract_and_store_package(path: &Path, tarball_bytes: &[u8]) -> io::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let tar = flate2::read::GzDecoder::new(tarball_bytes);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(temp_dir.path())?;

        fs::create_dir_all(path)?;

        let entries: Vec<_> = fs::read_dir(temp_dir.path())?.collect::<Result<Vec<_>, _>>()?;

        let extracted_package_dir = if entries.len() == 1 && entries[0].file_type()?.is_dir() {
            entries[0].path()
        } else {
            temp_dir.path().to_path_buf()
        };

        let final_package_dir = path.join("package");
        fs::create_dir_all(&final_package_dir)?;

        fs_extra::dir::copy(
            &extracted_package_dir,
            &final_package_dir,
            &fs_extra::dir::CopyOptions::new()
                .overwrite(true)
                .content_only(true),
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }
}

pub fn get_store_path() -> PathBuf {
    StoreManager::get_store_path()
}

pub fn store_package(
    package_name: &str,
    version: &str,
    tarball_bytes: &[u8],
) -> io::Result<PathBuf> {
    StoreManager::store_package(package_name, version, tarball_bytes)
}
