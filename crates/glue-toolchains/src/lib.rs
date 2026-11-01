use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error("No compatible native toolchain discovered on the host system")]
    NotFound,
    #[error("Toolchain execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolchainType {
    Msvc,
    Clang,
    Gcc,
}

pub trait Toolchain: Send + Sync {
    fn name(&self) -> &str;
    fn toolchain_type(&self) -> ToolchainType;
    fn compiler_path(&self) -> &Path;
    fn linker_path(&self) -> &Path;
    fn archiver_path(&self) -> &Path;

    fn compile_command(
        &self,
        source: &Path,
        output: &Path,
        include_dirs: &[PathBuf],
        defines: &[(String, Option<String>)],
        flags: &[String],
    ) -> Command;

    fn link_command(
        &self,
        objects: &[PathBuf],
        output: &Path,
        lib_dirs: &[PathBuf],
        libs: &[String],
        flags: &[String],
    ) -> Command;

    fn archive_command(
        &self,
        objects: &[PathBuf],
        output: &Path,
        flags: &[String],
    ) -> Command;

    fn generate_ninja_rules(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct MsvcToolchain {
    pub cl_path: PathBuf,
    pub link_path: PathBuf,
    pub lib_path: PathBuf,
    pub env_vars: HashMap<String, String>,
}

impl MsvcToolchain {
    pub fn discover() -> Option<Self> {
        let vs_base = PathBuf::from(r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC");
        if let Ok(entries) = std::fs::read_dir(&vs_base) {
            let mut versions: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            versions.sort();
            if let Some(latest_ver) = versions.last() {
                let bin = latest_ver.join(r"bin\Hostx64\x64");
                let cl = bin.join("cl.exe");
                let link = bin.join("link.exe");
                let lib = bin.join("lib.exe");
                if cl.exists() && link.exists() {
                    let mut env_vars = HashMap::new();
                    // Load environment variables via vcvarsall.bat if available
                    let vcvars = PathBuf::from(r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvarsall.bat");
                    if vcvars.exists() {
                        if let Ok(output) = Command::new("cmd")
                            .args(["/c", vcvars.to_str().unwrap(), "x64", "&&", "set"])
                            .output()
                        {
                            let env_text = String::from_utf8_lossy(&output.stdout);
                            for line in env_text.lines() {
                                if let Some((k, v)) = line.split_once('=') {
                                    env_vars.insert(k.trim().to_uppercase(), v.trim().to_string());
                                }
                            }
                        }
                    }

                    return Some(Self {
                        cl_path: cl,
                        link_path: link,
                        lib_path: lib,
                        env_vars,
                    });
                }
            }
        }
        None
    }

    fn apply_env(&self, cmd: &mut Command) {
        if !self.env_vars.is_empty() {
            for (k, v) in &self.env_vars {
                cmd.env(k, v);
            }
        }
    }
}

impl Toolchain for MsvcToolchain {
    fn name(&self) -> &str {
        "MSVC (Microsoft Visual C/C++)"
    }

    fn toolchain_type(&self) -> ToolchainType {
        ToolchainType::Msvc
    }

    fn compiler_path(&self) -> &Path {
        &self.cl_path
    }

    fn linker_path(&self) -> &Path {
        &self.link_path
    }

    fn archiver_path(&self) -> &Path {
        &self.lib_path
    }

    fn compile_command(
        &self,
        source: &Path,
        output: &Path,
        include_dirs: &[PathBuf],
        defines: &[(String, Option<String>)],
        flags: &[String],
    ) -> Command {
        let mut cmd = Command::new(&self.cl_path);
        self.apply_env(&mut cmd);
        cmd.arg("/nologo").arg("/c");
        for inc in include_dirs {
            cmd.arg(format!("/I{}", inc.display()));
        }
        for (k, v) in defines {
            if let Some(val) = v {
                cmd.arg(format!("/D{}={}", k, val));
            } else {
                cmd.arg(format!("/D{}", k));
            }
        }
        for f in flags {
            cmd.arg(f);
        }
        cmd.arg(source);
        cmd.arg(format!("/Fo{}", output.display()));
        cmd
    }

    fn link_command(
        &self,
        objects: &[PathBuf],
        output: &Path,
        lib_dirs: &[PathBuf],
        libs: &[String],
        flags: &[String],
    ) -> Command {
        let mut cmd = Command::new(&self.link_path);
        self.apply_env(&mut cmd);
        cmd.arg("/nologo");
        cmd.arg(format!("/OUT:{}", output.display()));
        for d in lib_dirs {
            cmd.arg(format!("/LIBPATH:{}", d.display()));
        }
        for obj in objects {
            cmd.arg(obj);
        }
        for lib in libs {
            cmd.arg(lib);
        }
        for f in flags {
            cmd.arg(f);
        }
        cmd
    }

    fn archive_command(
        &self,
        objects: &[PathBuf],
        output: &Path,
        flags: &[String],
    ) -> Command {
        let mut cmd = Command::new(&self.lib_path);
        self.apply_env(&mut cmd);
        cmd.arg("/nologo");
        cmd.arg(format!("/OUT:{}", output.display()));
        for obj in objects {
            cmd.arg(obj);
        }
        for f in flags {
            cmd.arg(f);
        }
        cmd
    }

    fn generate_ninja_rules(&self) -> String {
        format!(
            "rule cc\n  command = \"{}\" /nologo /showIncludes /EHsc /MD /O2 /c $in /Fo$out $FLAGS $INCLUDES\n  description = CC $out\n  deps = msvc\n\nrule link\n  command = \"{}\" /nologo /OUT:$out $in $FLAGS $LIBS\n  description = LINK $out\n\nrule archive\n  command = \"{}\" /nologo /OUT:$out $in $FLAGS\n  description = AR $out\n\n",
            self.cl_path.display(),
            self.link_path.display(),
            self.lib_path.display()
        )
    }
}

#[derive(Debug, Clone)]
pub struct ClangToolchain {
    pub clang_path: PathBuf,
    pub ar_path: PathBuf,
}

impl ClangToolchain {
    pub fn discover() -> Option<Self> {
        let ndk_llvm = PathBuf::from(r"C:\Users\User\Documents\globals\android-ndk-r30\toolchains\llvm\prebuilt\windows-x86_64\bin");
        let clang = ndk_llvm.join("clang.exe");
        let ar = ndk_llvm.join("llvm-ar.exe");
        if clang.exists() {
            return Some(Self {
                clang_path: clang,
                ar_path: if ar.exists() { ar } else { ndk_llvm.join("llvm-ar.exe") },
            });
        }
        None
    }
}

impl Toolchain for ClangToolchain {
    fn name(&self) -> &str {
        "Clang / LLVM"
    }

    fn toolchain_type(&self) -> ToolchainType {
        ToolchainType::Clang
    }

    fn compiler_path(&self) -> &Path {
        &self.clang_path
    }

    fn linker_path(&self) -> &Path {
        &self.clang_path
    }

    fn archiver_path(&self) -> &Path {
        &self.ar_path
    }

    fn compile_command(
        &self,
        source: &Path,
        output: &Path,
        include_dirs: &[PathBuf],
        defines: &[(String, Option<String>)],
        flags: &[String],
    ) -> Command {
        let mut cmd = Command::new(&self.clang_path);
        cmd.arg("-c").arg(source).arg("-o").arg(output);
        for inc in include_dirs {
            cmd.arg(format!("-I{}", inc.display()));
        }
        for (k, v) in defines {
            if let Some(val) = v {
                cmd.arg(format!("-D{}={}", k, val));
            } else {
                cmd.arg(format!("-D{}", k));
            }
        }
        for f in flags {
            cmd.arg(f);
        }
        cmd
    }

    fn link_command(
        &self,
        objects: &[PathBuf],
        output: &Path,
        lib_dirs: &[PathBuf],
        libs: &[String],
        flags: &[String],
    ) -> Command {
        let mut cmd = Command::new(&self.clang_path);
        cmd.arg("-o").arg(output);
        for obj in objects {
            cmd.arg(obj);
        }
        for d in lib_dirs {
            cmd.arg(format!("-L{}", d.display()));
        }
        for lib in libs {
            cmd.arg(format!("-l{}", lib));
        }
        for f in flags {
            cmd.arg(f);
        }
        cmd
    }

    fn archive_command(
        &self,
        objects: &[PathBuf],
        output: &Path,
        _flags: &[String],
    ) -> Command {
        let mut cmd = Command::new(&self.ar_path);
        cmd.arg("rcs").arg(output);
        for obj in objects {
            cmd.arg(obj);
        }
        cmd
    }

    fn generate_ninja_rules(&self) -> String {
        format!(
            "rule cc\n  command = \"{}\" -c $in -o $out -MMD -MF $out.d $FLAGS $INCLUDES\n  description = CC $out\n  depfile = $out.d\n  deps = gcc\n\nrule link\n  command = \"{}\" -o $out $in $FLAGS $LIBS\n  description = LINK $out\n\nrule archive\n  command = \"{}\" rcs $out $in\n  description = AR $out\n\n",
            self.clang_path.display(),
            self.clang_path.display(),
            self.ar_path.display()
        )
    }
}

pub struct ToolchainManager;

impl ToolchainManager {
    pub fn detect_best_toolchain() -> Result<Box<dyn Toolchain>, ToolchainError> {
        if let Some(msvc) = MsvcToolchain::discover() {
            return Ok(Box::new(msvc));
        }
        if let Some(clang) = ClangToolchain::discover() {
            return Ok(Box::new(clang));
        }
        Err(ToolchainError::NotFound)
    }
}
