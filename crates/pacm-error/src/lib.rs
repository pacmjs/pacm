use std::fmt;

#[derive(Debug)]
pub enum PackageManagerError {
    PackageNotFound(String),
    VersionResolutionFailed(String, String),
    DownloadFailed(String, String),
    StorageFailed(String, String),
    LinkingFailed(String, String),
    LockfileError(String),
    PackageJsonError(String),
    PackageJsonExists(String),
    NetworkError(String),
    InvalidPackageSpec(String),
    DependencyConflict(String, String),
    IoError(String),
}

impl fmt::Display for PackageManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackageJsonExists(path) => {
                write!(f, "Package.json already exists at {path}")
            }
            Self::PackageNotFound(name) => {
                write!(f, "Package '{name}' not found")
            }
            Self::VersionResolutionFailed(name, range) => {
                write!(f, "Failed to resolve version for {name}@{range}")
            }
            Self::DownloadFailed(name, version) => {
                write!(f, "Failed to download {name}@{version}")
            }
            Self::StorageFailed(name, version) => {
                write!(f, "Failed to store {name}@{version}")
            }
            Self::LinkingFailed(name, reason) => {
                write!(f, "Failed to link package '{name}': {reason}")
            }
            Self::LockfileError(msg) => {
                write!(f, "Lockfile error: {msg}")
            }
            Self::PackageJsonError(msg) => {
                write!(f, "Package.json error: {msg}")
            }
            Self::NetworkError(msg) => {
                write!(f, "Network error: {msg}")
            }
            Self::InvalidPackageSpec(spec) => {
                write!(f, "Invalid package specification: {spec}")
            }
            Self::DependencyConflict(name, details) => {
                write!(f, "Dependency conflict for '{name}': {details}")
            }
            Self::IoError(msg) => {
                write!(f, "IO error: {msg}")
            }
        }
    }
}

impl std::error::Error for PackageManagerError {}

impl From<anyhow::Error> for PackageManagerError {
    fn from(err: anyhow::Error) -> Self {
        Self::PackageJsonError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PackageManagerError>;
