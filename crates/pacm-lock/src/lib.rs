use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct LockDependency {
    pub version: String,
    pub resolved: String,
    pub integrity: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PacmLock {
    pub dependencies: HashMap<String, LockDependency>,
}

impl PacmLock {
    pub fn load(path: &Path) -> io::Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn update_dep(&mut self, name: &str, dep: LockDependency) {
        self.dependencies.insert(name.to_string(), dep);
    }

    #[must_use]
    pub fn get_dependency(&self, name: &str) -> Option<&LockDependency> {
        self.dependencies.get(name)
    }

    pub fn remove_dep(&mut self, name: &str) {
        self.dependencies
            .retain(|key, _| !key.starts_with(&format!("{name}@")));
    }

    pub fn remove_dep_exact(&mut self, key: &str) {
        self.dependencies.remove(key);
    }

    #[must_use]
    pub fn has_all_dependencies(&self, required_deps: &[String]) -> bool {
        required_deps
            .iter()
            .all(|dep| self.dependencies.contains_key(dep))
    }
}
