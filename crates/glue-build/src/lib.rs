use glue_model::{Project, Target};
use glue_toolchains::{Toolchain, ToolchainError};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Toolchain error: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("Compilation failed for target '{target}': {message}")]
    CompilationFailed { target: String, message: String },
    #[error("Linking failed for target '{target}': {message}")]
    LinkingFailed { target: String, message: String },
    #[error("Target '{0}' not found in project")]
    TargetNotFound(String),
    #[error("Circular dependency detected involving target '{0}'")]
    CircularDependency(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct CompileUnit {
    pub source: PathBuf,
    pub object: PathBuf,
    pub include_dirs: Vec<PathBuf>,
    pub defines: Vec<(String, Option<String>)>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TargetPlan {
    pub target: Target,
    pub compile_units: Vec<CompileUnit>,
    pub output_path: PathBuf,
    pub dependent_lib_paths: Vec<PathBuf>,
}

pub struct BuildPlanner<'a> {
    project: &'a Project,
    toolchain: &'a dyn Toolchain,
    build_dir: PathBuf,
}

impl<'a> BuildPlanner<'a> {
    pub fn new(project: &'a Project, toolchain: &'a dyn Toolchain) -> Self {
        let build_dir = project.root_dir.join(".glue").join("build");
        Self {
            project,
            toolchain,
            build_dir,
        }
    }

    pub fn build_dir(&self) -> &Path {
        &self.build_dir
    }

    pub fn plan_transitive(&self, target_name: &str) -> Result<Vec<TargetPlan>, BuildError> {
        let mut plans = Vec::new();
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();
        self.collect_transitive(target_name, &mut plans, &mut visited, &mut in_progress)?;
        Ok(plans)
    }

    fn collect_transitive(
        &self,
        target_name: &str,
        plans: &mut Vec<TargetPlan>,
        visited: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
    ) -> Result<(), BuildError> {
        if visited.contains(target_name) {
            return Ok(());
        }
        if !in_progress.insert(target_name.to_string()) {
            return Err(BuildError::CircularDependency(target_name.to_string()));
        }

        let target = self
            .project
            .get_target(target_name)
            .ok_or_else(|| BuildError::TargetNotFound(target_name.to_string()))?;

        for dep in &target.dependencies {
            self.collect_transitive(dep, plans, visited, in_progress)?;
        }

        in_progress.remove(target_name);
        visited.insert(target_name.to_string());

        let plan = self.plan_target(target_name)?;
        plans.push(plan);
        Ok(())
    }

    pub fn plan_target(&self, target_name: &str) -> Result<TargetPlan, BuildError> {
        let target = self
            .project
            .get_target(target_name)
            .ok_or_else(|| BuildError::TargetNotFound(target_name.to_string()))?;

        let mut visited = HashSet::new();
        self.check_circular(target_name, &mut visited)?;

        let mut propagated_includes = Vec::new();
        let mut dependent_lib_paths = Vec::new();

        // Collect propagated requirements and libraries from dependencies
        for dep_name in &target.dependencies {
            if let Some(dep) = self.project.get_target(dep_name) {
                // Public include dirs from dependencies are propagated to consumers
                for inc in &dep.include_dirs_public {
                    propagated_includes.push(self.project.root_dir.join(inc));
                }

                let dep_bin_name = dep.binary_name(cfg!(windows));
                let dep_out = self.build_dir.join("bin").join(dep_bin_name);
                dependent_lib_paths.push(dep_out);
            }
        }

        let mut all_includes = Vec::new();
        for inc in &target.include_dirs_private {
            all_includes.push(self.project.root_dir.join(inc));
        }
        for inc in &target.include_dirs_public {
            all_includes.push(self.project.root_dir.join(inc));
        }
        all_includes.extend(propagated_includes);

        let target_obj_dir = self.build_dir.join("obj").join(&target.name);
        let mut compile_units = Vec::new();

        for src in &target.sources {
            let full_src = if src.is_absolute() {
                src.clone()
            } else {
                self.project.root_dir.join(src)
            };

            let stem = full_src.file_name().unwrap().to_str().unwrap();
            let obj_name = if cfg!(windows) && self.toolchain.toolchain_type() == glue_toolchains::ToolchainType::Msvc {
                format!("{}.obj", stem)
            } else {
                format!("{}.o", stem)
            };
            let obj_path = target_obj_dir.join(obj_name);

            compile_units.push(CompileUnit {
                source: full_src,
                object: obj_path,
                include_dirs: all_includes.clone(),
                defines: target.defines.clone(),
                flags: target.compile_flags.clone(),
            });
        }

        let bin_name = target.binary_name(cfg!(windows));
        let output_path = self.build_dir.join("bin").join(bin_name);

        Ok(TargetPlan {
            target: target.clone(),
            compile_units,
            output_path,
            dependent_lib_paths,
        })
    }

    fn check_circular(&self, target_name: &str, visited: &mut HashSet<String>) -> Result<(), BuildError> {
        if !visited.insert(target_name.to_string()) {
            return Err(BuildError::CircularDependency(target_name.to_string()));
        }

        if let Some(target) = self.project.get_target(target_name) {
            for dep in &target.dependencies {
                self.check_circular(dep, visited)?;
            }
        }

        visited.remove(target_name);
        Ok(())
    }

    pub fn compute_source_fingerprint(source_path: &Path) -> Result<String, std::io::Error> {
        let content = std::fs::read(source_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }
}
