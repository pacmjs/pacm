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
}

impl fmt::Display for PackageManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManagerError::PackageJsonExists(path) => {
                write!(f, "Package.json already exists at {}", path)
            }
            PackageManagerError::PackageNotFound(name) => {
                write!(f, "Package '{}' not found", name)
            }
            PackageManagerError::VersionResolutionFailed(name, range) => {
                write!(f, "Failed to resolve version for {}@{}", name, range)
            }
            PackageManagerError::DownloadFailed(name, version) => {
                write!(f, "Failed to download {}@{}", name, version)
            }
            PackageManagerError::StorageFailed(name, version) => {
                write!(f, "Failed to store {}@{}", name, version)
            }
            PackageManagerError::LinkingFailed(name, reason) => {
                write!(f, "Failed to link package '{}': {}", name, reason)
            }
            PackageManagerError::LockfileError(msg) => {
                write!(f, "Lockfile error: {}", msg)
            }
            PackageManagerError::PackageJsonError(msg) => {
                write!(f, "Package.json error: {}", msg)
            }
            PackageManagerError::NetworkError(msg) => {
                write!(f, "Network error: {}", msg)
            }
            PackageManagerError::InvalidPackageSpec(spec) => {
                write!(f, "Invalid package specification: {}", spec)
            }
            PackageManagerError::DependencyConflict(name, details) => {
                write!(f, "Dependency conflict for '{}': {}", name, details)
            }
        }
    }
}

impl std::error::Error for PackageManagerError {}

impl From<anyhow::Error> for PackageManagerError {
    fn from(err: anyhow::Error) -> Self {
        PackageManagerError::PackageJsonError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PackageManagerError>;
