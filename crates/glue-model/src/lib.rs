use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetKind {
    Exec,
    StaticLib,
    SharedLib,
    ObjectLib,
    InterfaceLib,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub name: String,
    pub namespace: Option<String>,
    pub kind: TargetKind,
    pub sources: Vec<PathBuf>,
    pub include_dirs_public: Vec<PathBuf>,
    pub include_dirs_private: Vec<PathBuf>,
    pub defines: Vec<(String, Option<String>)>,
    pub dependencies: Vec<String>,
    pub compile_flags: Vec<String>,
    pub link_flags: Vec<String>,
    pub output_name: Option<String>,
    /// Path to a prebuilt library when this target is imported from a package.
    pub prebuilt_artifact: Option<PathBuf>,
    /// Runtime artifacts (e.g. DLLs, shared objects) required at execution time.
    #[serde(default)]
    pub runtime_artifacts: Vec<PathBuf>,
    /// Source/include root for a target compiled from an imported source package.
    pub source_root: Option<PathBuf>,
    /// Original qualified name before this package was imported through an alias.
    pub source_target: Option<String>,
}

impl Target {
    pub fn new(name: impl Into<String>, kind: TargetKind) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            kind,
            sources: Vec::new(),
            include_dirs_public: Vec::new(),
            include_dirs_private: Vec::new(),
            defines: Vec::new(),
            dependencies: Vec::new(),
            compile_flags: Vec::new(),
            link_flags: Vec::new(),
            output_name: None,
            prebuilt_artifact: None,
            runtime_artifacts: Vec::new(),
            source_root: None,
            source_target: None,
        }
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn qualified_name(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}::{}", ns, self.name),
            None => self.name.clone(),
        }
    }

    pub fn binary_name(&self, host_is_windows: bool) -> String {
        let base = self.output_name.as_deref().unwrap_or(&self.name);
        match self.kind {
            TargetKind::Exec => {
                if host_is_windows {
                    format!("{}.exe", base)
                } else {
                    base.to_string()
                }
            }
            TargetKind::StaticLib => {
                if host_is_windows {
                    format!("{}.lib", base)
                } else {
                    format!("lib{}.a", base)
                }
            }
            TargetKind::SharedLib => {
                if host_is_windows {
                    format!("{}.dll", base)
                } else {
                    format!("lib{}.so", base)
                }
            }
            TargetKind::ObjectLib | TargetKind::InterfaceLib | TargetKind::Custom => {
                base.to_string()
            }
        }
    }

    pub fn link_library_name(&self, host_is_windows: bool) -> String {
        let base = self.output_name.as_deref().unwrap_or(&self.name);
        match self.kind {
            TargetKind::StaticLib => {
                if host_is_windows {
                    format!("{}.lib", base)
                } else {
                    format!("lib{}.a", base)
                }
            }
            TargetKind::SharedLib => {
                if host_is_windows {
                    format!("{}.lib", base)
                } else {
                    format!("lib{}.so", base)
                }
            }
            _ => self.binary_name(host_is_windows),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildProfile {
    Debug,
    Release,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedPackageRecord {
    pub alias: String,
    pub source: String,
    pub resolved_path: PathBuf,
    pub version_req: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub root_dir: PathBuf,
    pub targets: HashMap<String, Target>,
    pub default_target: Option<String>,
    #[serde(default)]
    pub imported_packages: Vec<ImportedPackageRecord>,
}

impl Project {
    pub fn new(name: impl Into<String>, root_dir: PathBuf) -> Self {
        Self {
            name: name.into(),
            root_dir,
            targets: HashMap::new(),
            default_target: None,
            imported_packages: Vec::new(),
        }
    }

    pub fn add_target(&mut self, target: Target) {
        let qualified = target.qualified_name();
        if self.default_target.is_none() {
            self.default_target = Some(qualified.clone());
        }
        self.targets.insert(qualified, target.clone());
        if target.namespace.is_some() {
            self.targets.entry(target.name.clone()).or_insert(target);
        }
    }

    pub fn get_target(&self, name: &str) -> Option<&Target> {
        if let Some(t) = self.targets.get(name) {
            return Some(t);
        }
        // If not found and name does not have namespace, look for target with matching name
        if !name.contains("::") {
            for target in self.targets.values() {
                if target.name == name {
                    return Some(target);
                }
            }
        }
        None
    }
}
