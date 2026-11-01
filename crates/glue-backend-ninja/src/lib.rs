use glue_build::{BuildError, TargetPlan};
use glue_model::TargetKind;
use glue_toolchains::Toolchain;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NinjaError {
    #[error("Ninja executable not found in PATH or standard locations")]
    NinjaNotFound,
    #[error("Ninja build failed with exit code {0}: {1}")]
    ExecutionFailed(i32, String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Build error: {0}")]
    Build(#[from] BuildError),
}

fn escape_ninja_path(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let mut out = String::new();
    for c in s.chars() {
        match c {
            ':' => out.push_str("$:"),
            ' ' => out.push_str("$ "),
            '$' => out.push_str("$$"),
            other => out.push(other),
        }
    }
    out
}

pub struct NinjaBackend<'a> {
    toolchain: &'a dyn Toolchain,
    ninja_path: PathBuf,
}

impl<'a> NinjaBackend<'a> {
    pub fn new(toolchain: &'a dyn Toolchain) -> Result<Self, NinjaError> {
        let candidates = [
            PathBuf::from("ninja.exe"),
            PathBuf::from("ninja"),
            PathBuf::from(r"C:\Users\User\Documents\globals\ninja.exe"),
        ];

        let mut found = None;
        for c in &candidates {
            if c.is_file() {
                found = Some(c.clone());
                break;
            }
            if let Ok(output) = Command::new(c).arg("--version").output() {
                if output.status.success() {
                    found = Some(c.clone());
                    break;
                }
            }
        }

        // If which crate is not present, check directly with Command
        if found.is_none() {
            if let Ok(output) = Command::new("ninja").arg("--version").output() {
                if output.status.success() {
                    found = Some(PathBuf::from("ninja"));
                }
            }
        }

        // Check fallback direct global path
        if found.is_none() {
            let global = PathBuf::from(r"C:\Users\User\Documents\globals\ninja.exe");
            if global.exists() {
                found = Some(global);
            }
        }

        let ninja_path = found.ok_or(NinjaError::NinjaNotFound)?;

        Ok(Self {
            toolchain,
            ninja_path,
        })
    }

    pub fn generate_ninja_file(
        &self,
        plans: &[TargetPlan],
        build_dir: &Path,
    ) -> Result<PathBuf, NinjaError> {
        fs::create_dir_all(build_dir.join("bin"))?;
        let ninja_file = build_dir.join("build.ninja");
        let mut content = String::new();

        content.push_str("ninja_required_version = 1.3\n\n");
        content.push_str(&self.toolchain.generate_ninja_rules());

        for plan in plans {
            let sanitized_name = plan.target.name.replace("::", "_");
            fs::create_dir_all(build_dir.join("obj").join(sanitized_name))?;

            let mut obj_paths = Vec::new();

            for unit in &plan.compile_units {
                let mut inc_flags = String::new();
                for inc in &unit.include_dirs {
                    if self.toolchain.toolchain_type() == glue_toolchains::ToolchainType::Msvc {
                        inc_flags.push_str(&format!(" \"/I{}\"", inc.display()));
                    } else {
                        inc_flags.push_str(&format!(" \"-I{}\"", inc.display()));
                    }
                }

                let mut def_flags = String::new();
                for (k, v) in &unit.defines {
                    if self.toolchain.toolchain_type() == glue_toolchains::ToolchainType::Msvc {
                        if let Some(val) = v {
                            def_flags.push_str(&format!(" /D{}={}", k, val));
                        } else {
                            def_flags.push_str(&format!(" /D{}", k));
                        }
                    } else {
                        if let Some(val) = v {
                            def_flags.push_str(&format!(" -D{}={}", k, val));
                        } else {
                            def_flags.push_str(&format!(" -D{}", k));
                        }
                    }
                }

                let mut extra_flags = String::new();
                for f in &unit.flags {
                    extra_flags.push_str(&format!(" {}", f));
                }

                content.push_str(&format!(
                    "build {}: cc {}\n  INCLUDES ={}\n  FLAGS = {}{}\n\n",
                    escape_ninja_path(&unit.object),
                    escape_ninja_path(&unit.source),
                    inc_flags,
                    def_flags,
                    extra_flags
                ));

                obj_paths.push(escape_ninja_path(&unit.object));
            }

            let rule = match plan.target.kind {
                TargetKind::Exec | TargetKind::SharedLib => "link",
                TargetKind::StaticLib => "archive",
                _ => "link",
            };

            let mut implicit_deps = String::new();
            if !plan.dependent_lib_paths.is_empty() {
                implicit_deps.push_str(" |");
                for dep in &plan.dependent_lib_paths {
                    implicit_deps.push_str(&format!(" {}", escape_ninja_path(dep)));
                }
            }

            let mut libs_flags = String::new();
            for dep in &plan.dependent_lib_paths {
                libs_flags.push_str(&format!(" \"{}\"", dep.display()));
            }

            content.push_str(&format!(
                "build {}: {} {}{}\n  LIBS ={}\n\n",
                escape_ninja_path(&plan.output_path),
                rule,
                obj_paths.join(" "),
                implicit_deps,
                libs_flags
            ));
        }

        if let Some(root_plan) = plans.last() {
            content.push_str(&format!("default {}\n", escape_ninja_path(&root_plan.output_path)));
        }

        fs::write(&ninja_file, content)?;
        Ok(ninja_file)
    }

    pub fn build(&self, ninja_file: &Path) -> Result<(), NinjaError> {
        let parent = ninja_file.parent().unwrap_or(Path::new("."));
        let mut cmd = Command::new(&self.ninja_path);
        cmd.current_dir(parent);
        cmd.arg("-f").arg(ninja_file.file_name().unwrap());

        // Apply MSVC environment if needed
        if let glue_toolchains::ToolchainType::Msvc = self.toolchain.toolchain_type() {
            if let Some(msvc) = glue_toolchains::MsvcToolchain::discover() {
                for (k, v) in &msvc.env_vars {
                    cmd.env(k, v);
                }
            }
        }

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(NinjaError::ExecutionFailed(
                output.status.code().unwrap_or(-1),
                format!("{}\n{}", stdout, stderr),
            ));
        }

        Ok(())
    }
}
