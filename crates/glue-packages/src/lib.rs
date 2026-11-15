use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("Failed to parse package manifest: {0}")]
    ManifestParse(#[from] toml::de::Error),
    #[error("Failed to serialize package manifest: {0}")]
    ManifestSerialize(#[from] toml::ser::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Package not found: {0}")]
    NotFound(String),
    #[error("Invalid package product: {0}")]
    InvalidProduct(String),
    #[error("Package integrity check failed at '{path}': expected {expected}, found {actual}")]
    Integrity { path: String, expected: String, actual: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIdentity {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub source: PackageSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageSource {
    Github { repo: String },
    Git { url: String },
    Vendor { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lockfile {
    pub version: u32,
    #[serde(default)]
    pub package: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedPackage {
    pub coordinate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_digest: Option<String>,
    #[serde(default = "default_delivery_mode")]
    pub delivery_mode: String,
}

fn default_delivery_mode() -> String {
    "source_built".to_string()
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            version: 1,
            package: Vec::new(),
        }
    }

    pub fn parse(content: &str) -> Result<Self, PackageError> {
        let lock: Lockfile = toml::from_str(content)?;
        Ok(lock)
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self, PackageError> {
        let path = dir.join("glue.lock");
        if !path.exists() {
            return Err(PackageError::NotFound("glue.lock".to_string()));
        }
        let content = fs::read_to_string(&path)?;
        Self::parse(&content)
    }

    pub fn write_to_dir(&self, dir: &Path) -> Result<PathBuf, PackageError> {
        fs::create_dir_all(dir)?;
        let path = dir.join("glue.lock");
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: PackageMetadata,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub variant: Option<PackageVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVariant {
    pub id: String,
    pub target_os: String,
    pub target_arch: String,
    pub toolchain: String,
    pub toolchain_fingerprint: String,
    pub target_name: String,
    pub target_namespace: Option<String>,
    pub target_kind: String,
    pub artifact: PathBuf,
    #[serde(default)]
    pub runtime_artifacts: Vec<PathBuf>,
    #[serde(default)]
    pub include_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub source_project: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
}

impl PackageManifest {
    pub fn parse(content: &str) -> Result<Self, PackageError> {
        let manifest: PackageManifest = toml::from_str(content)?;
        Ok(manifest)
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self, PackageError> {
        let content = fs::read_to_string(dir.join("package.toml"))?;
        Self::parse(&content)
    }

    pub fn write_to_dir(&self, dir: &Path) -> Result<PathBuf, PackageError> {
        fs::create_dir_all(dir)?;
        let path = dir.join("package.toml");
        fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(path)
    }

    /// Hashes a package tree in sorted relative-path order, excluding the detached digest file.
    pub fn content_digest(dir: &Path) -> Result<String, PackageError> {
        let mut files = Vec::new();
        collect_files(dir, dir, &mut files)?;
        files.sort();

        let mut hasher = Sha256::new();
        for path in files {
            let relative = path.strip_prefix(dir).unwrap_or(&path);
            let normalized = relative.to_string_lossy().replace('\\', "/");
            let bytes = fs::read(&path)?;
            hasher.update((normalized.len() as u64).to_le_bytes());
            hasher.update(normalized.as_bytes());
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(bytes);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn write_content_digest(dir: &Path) -> Result<String, PackageError> {
        let digest = Self::content_digest(dir)?;
        fs::write(dir.join("content.sha256"), format!("{}\n", digest))?;
        Ok(digest)
    }

    pub fn verify_content_digest(dir: &Path) -> Result<String, PackageError> {
        let expected = fs::read_to_string(dir.join("content.sha256"))?.trim().to_string();
        let actual = Self::content_digest(dir)?;
        if expected != actual {
            return Err(PackageError::Integrity {
                path: dir.display().to_string(),
                expected,
                actual,
            });
        }
        Ok(actual)
    }

    pub fn toolchain_fingerprint(paths: &[PathBuf]) -> Result<String, PackageError> {
        let mut hasher = Sha256::new();
        for path in paths {
            if !path.is_file() {
                return Err(PackageError::InvalidProduct(format!("Toolchain component is not a file: {}", path.display())));
            }
            let bytes = fs::read(path)?;
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(bytes);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
}

fn collect_files(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> Result<(), PackageError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(PackageError::InvalidProduct(format!(
                "Package products cannot contain symlinks: {}",
                path.display()
            )));
        }
        if kind.is_dir() {
            collect_files(root, &path, output)?;
        } else if kind.is_file() {
            if !path.starts_with(root) {
                return Err(PackageError::InvalidProduct(format!("Path escapes package root: {}", path.display())));
            }
            let relative = path.strip_prefix(root).unwrap_or(&path);
            if relative != Path::new("content.sha256") {
                output.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_roundtrip() {
        let mut lock = Lockfile::new();
        lock.package.push(LockedPackage {
            coordinate: "github@pinwhell/inih".to_string(),
            version: Some("0.5.0".to_string()),
            source_revision: Some("abcdef123456".to_string()),
            resolved_variant: Some("x86_64-linux-gnu-static".to_string()),
            variant_id: Some("var_1234".to_string()),
            content_digest: Some("sha256:7b9260c6d59e35d259c735d46c646b9a".to_string()),
            delivery_mode: "binary_download".to_string(),
        });

        let toml_str = toml::to_string_pretty(&lock).expect("serialize lockfile");
        let parsed = Lockfile::parse(&toml_str).expect("deserialize lockfile");
        assert_eq!(lock, parsed);
    }
}
