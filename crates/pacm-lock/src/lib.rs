use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct LockDependency {
    pub version: String,
    pub resolved: String,
    pub integrity: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LockPackage {
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub dependencies: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub optional_dependencies: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceInfo {
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub dependencies: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub peer_dependencies: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub optional_dependencies: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PacmLock {
    #[serde(rename = "lockfileVersion")]
    pub lockfile_version: u32,
    pub workspaces: HashMap<String, WorkspaceInfo>,
    pub packages: HashMap<String, LockPackage>,

    // Legacy field for backward compatibility
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub dependencies: HashMap<String, LockDependency>,
}

impl Default for PacmLock {
    fn default() -> Self {
        Self {
            lockfile_version: 1,
            workspaces: {
                let mut map = HashMap::new();
                map.insert(
                    String::new(),
                    WorkspaceInfo {
                        dependencies: HashMap::new(),
                        dev_dependencies: HashMap::new(),
                        peer_dependencies: HashMap::new(),
                        optional_dependencies: HashMap::new(),
                    },
                );
                map
            },
            packages: HashMap::new(),
            dependencies: HashMap::new(), // Legacy field
        }
    }
}

impl PacmLock {
    pub fn load(path: &Path) -> io::Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let mut lockfile: Self = serde_json::from_str(&content)?;

            if !lockfile.dependencies.is_empty() && lockfile.packages.is_empty() {
                lockfile.migrate_from_legacy();
            }

            Ok(lockfile)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn migrate_from_legacy(&mut self) {
        for (key, legacy_dep) in &self.dependencies {
            if let Some(at_pos) = key.rfind('@') {
                let package_name = &key[..at_pos];
                self.packages.insert(
                    package_name.to_string(),
                    LockPackage {
                        version: legacy_dep.version.clone(),
                        resolved: legacy_dep.resolved.clone(),
                        integrity: legacy_dep.integrity.clone(),
                        dependencies: HashMap::new(),
                        optional_dependencies: HashMap::new(),
                    },
                );
            }
        }
        self.dependencies.clear();
    }

    pub fn update_workspace_deps(
        &mut self,
        workspace: &str,
        deps: &HashMap<String, String>,
        dep_type: &str,
    ) {
        let workspace_info = self
            .workspaces
            .entry(workspace.to_string())
            .or_insert_with(|| WorkspaceInfo {
                dependencies: HashMap::new(),
                dev_dependencies: HashMap::new(),
                peer_dependencies: HashMap::new(),
                optional_dependencies: HashMap::new(),
            });

        match dep_type {
            "dependencies" => workspace_info.dependencies.extend(deps.clone()),
            "devDependencies" => workspace_info.dev_dependencies.extend(deps.clone()),
            "peerDependencies" => workspace_info.peer_dependencies.extend(deps.clone()),
            "optionalDependencies" => workspace_info.optional_dependencies.extend(deps.clone()),
            _ => workspace_info.dependencies.extend(deps.clone()),
        }
    }

    pub fn update_package(&mut self, name: &str, package: LockPackage) {
        self.packages.insert(name.to_string(), package);
    }

    pub fn update_dep(&mut self, name: &str, dep: LockDependency) {
        if let Some(at_pos) = name.rfind('@') {
            let package_name = &name[..at_pos];
            self.packages.insert(
                package_name.to_string(),
                LockPackage {
                    version: dep.version,
                    resolved: dep.resolved,
                    integrity: dep.integrity,
                    dependencies: HashMap::new(),
                    optional_dependencies: HashMap::new(),
                },
            );
        }
    }

    #[must_use]
    pub fn get_dependency(&self, name: &str) -> Option<&LockDependency> {
        self.dependencies.get(name)
    }

    #[must_use]
    pub fn get_package(&self, name: &str) -> Option<&LockPackage> {
        self.packages.get(name)
    }

    pub fn remove_dep(&mut self, name: &str) {
        self.packages.remove(name);

        for workspace_info in self.workspaces.values_mut() {
            workspace_info.dependencies.remove(name);
            workspace_info.dev_dependencies.remove(name);
            workspace_info.peer_dependencies.remove(name);
            workspace_info.optional_dependencies.remove(name);
        }

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
            .all(|dep| self.packages.contains_key(dep) || self.dependencies.contains_key(dep))
    }

    pub fn get_all_packages(&self) -> &HashMap<String, LockPackage> {
        &self.packages
    }

    pub fn remove_workspace_dep(&mut self, workspace: &str, name: &str) {
        if let Some(workspace_info) = self.workspaces.get_mut(workspace) {
            workspace_info.dependencies.remove(name);
            workspace_info.dev_dependencies.remove(name);
            workspace_info.peer_dependencies.remove(name);
            workspace_info.optional_dependencies.remove(name);
        }
    }
}
