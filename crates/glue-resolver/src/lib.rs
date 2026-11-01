use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("Failed to parse lockfile: {0}")]
    LockfileParse(#[from] toml::de::Error),
    #[error("Failed to serialize lockfile: {0}")]
    LockfileSerialize(#[from] toml::ser::Error),
    #[error("Unsatisfied version requirement for package '{0}'")]
    UnsatisfiedVersion(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub revision: Option<String>,
    pub checksum: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    #[serde(default)]
    pub packages: HashMap<String, LockPackage>,
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            version: 1,
            packages: HashMap::new(),
        }
    }

    pub fn load_from_dir(dir: &Path) -> Result<Option<Self>, ResolverError> {
        let lock_path = dir.join("glue.lock");
        if !lock_path.is_file() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(lock_path)?;
        let lockfile: Lockfile = toml::from_str(&content)?;
        Ok(Some(lockfile))
    }

    pub fn write_to_dir(&self, dir: &Path) -> Result<PathBuf, ResolverError> {
        let lock_path = dir.join("glue.lock");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&lock_path, content)?;
        Ok(lock_path)
    }
}
