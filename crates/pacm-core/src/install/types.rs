use pacm_resolver::ResolvedPackage;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CachedPackage {
    pub name: String,
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub store_path: PathBuf,
}

#[derive(Debug)]
pub enum PackageSource {
    Cache(CachedPackage),
    Download(ResolvedPackage),
}
