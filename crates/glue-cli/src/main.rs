use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use glue_backend_ninja::NinjaBackend;
use glue_build::{BuildPlanner, TargetPlan};
use glue_config::find_and_parse_project;
use glue_model::TargetKind;
use glue_packages::{Lockfile, LockedPackage, PackageManifest, PackageMetadata, PackageVariant};
use glue_toolchains::ToolchainManager;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Parser, Debug)]
#[command(
    name = "glue",
    about = "Glue - A Unified Native Development System",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new Glue project
    Init {
        /// Target name for the project
        #[arg(default_value = "main")]
        name: String,
    },
    /// Build project targets
    Build {
        /// Specific target to build (defaults to first declared target)
        target: Option<String>,
        /// Verify the build against glue.lock without updating it
        #[arg(long)]
        locked: bool,
    },
    /// Build and run an executable target
    Run {
        /// Target to run
        target: Option<String>,
        /// Arguments forwarded to the target
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build and write a relocatable local package product
    Package {
        /// Target to package (defaults to the first declared target)
        target: Option<String>,
        /// Package version recorded in the product
        #[arg(long, default_value = "0.1.0")]
        version: String,
    },
    /// Remove build artifacts (product/build directory)
    Clean {
        /// Remove all products including packages and scratch cache
        #[arg(long)]
        all: bool,
    },
    /// Inspect target declarations and properties
    Explain {
        /// Target to inspect
        target: Option<String>,
    },
    /// Print dependency and build graph
    Graph {
        /// Target to graph
        target: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run_command(cli) {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("{}: {:#}", "glue error".red().bold(), err);
            ExitCode::from(1)
        }
    }
}

fn run_command(cli: Cli) -> Result<i32> {
    match cli.command {
        Commands::Init { name } => {
            let here_glue = Path::new("here.glue");
            let gluefile = Path::new("gluefile");
            if here_glue.exists() || gluefile.exists() {
                anyhow::bail!("A Glue project already exists in this directory");
            }

            fs::write(
                "here.glue",
                format!("exec.add({}).sources(\"main.cpp\")\n", name),
            )?;
            fs::write(
                "main.cpp",
                "#include <iostream>\n\nint main() {\n    std::cout << \"Hello from Glue!\" << std::endl;\n    return 0;\n}\n",
            )?;

            println!(
                "{}: Initialized new Glue project with target '{}'",
                "glue".cyan().bold(),
                name
            );
            Ok(0)
        }
        Commands::Build { target, locked } => {
            let curr_dir = std::env::current_dir()?;
            let project = find_and_parse_project(&curr_dir)
                .context("Failed to load Glue project")?;

            if locked {
                let existing_lock = Lockfile::load_from_dir(&curr_dir)
                    .context("--locked specified, but no glue.lock file exists in project directory")?;
                for pkg in &project.imported_packages {
                    let matching = existing_lock.package.iter().find(|p| p.coordinate == pkg.source);
                    if matching.is_none() {
                        anyhow::bail!(
                            "Frozen lockfile mismatch: package '{}' ({}) is not recorded in glue.lock",
                            pkg.alias,
                            pkg.source
                        );
                    }
                }
                println!("{}: Verified dependency resolution against glue.lock", "glue".cyan().bold());
            } else if !project.imported_packages.is_empty() {
                let mut lock = Lockfile::new();
                for pkg in &project.imported_packages {
                    let is_prebuilt = project.targets.values().any(|t| {
                        t.namespace.as_deref() == Some(&pkg.alias) && t.prebuilt_artifact.is_some()
                    });
                    lock.package.push(LockedPackage {
                        coordinate: pkg.source.clone(),
                        version: pkg.version_req.clone(),
                        source_revision: None,
                        resolved_variant: None,
                        variant_id: None,
                        content_digest: None,
                        delivery_mode: if is_prebuilt { "local_product".to_string() } else { "source_built".to_string() },
                    });
                }
                let _ = lock.write_to_dir(&curr_dir);
            }

            let target_name = target
                .as_deref()
                .or(project.default_target.as_deref())
                .ok_or_else(|| anyhow::anyhow!("No target specified and no default target found"))?;

            let toolchain = ToolchainManager::detect_best_toolchain()
                .context("Failed to detect compatible native toolchain")?;

            println!(
                "{}: Building target '{}' using {}",
                "glue".cyan().bold(),
                target_name,
                toolchain.name()
            );

            let planner = BuildPlanner::new(&project, toolchain.as_ref());
            let plans = planner.plan_transitive(target_name)?;
            let root_plan = plans.last().unwrap();

            let ninja = NinjaBackend::new(toolchain.as_ref())?;
            let ninja_file = ninja.generate_ninja_file(&plans, planner.build_dir())?;
            ninja.build(&ninja_file)?;
            cache_source_fallbacks(&project, &plans);
            let deployed = planner.deploy_runtime_artifacts(target_name)?;
            for d in deployed {
                println!(
                    "{}: Staged runtime artifact: {}",
                    "glue".cyan().bold(),
                    d.file_name().unwrap().to_string_lossy()
                );
            }

            println!(
                "{}: Successfully built artifact: {}",
                "glue".green().bold(),
                root_plan.output_path.display()
            );
            Ok(0)
        }
        Commands::Run { target, args } => {
            let curr_dir = std::env::current_dir()?;
            let project = find_and_parse_project(&curr_dir)
                .context("Failed to load Glue project")?;
            let target_name = target
                .as_deref()
                .or(project.default_target.as_deref())
                .ok_or_else(|| anyhow::anyhow!("No target specified and no default target found"))?;

            let toolchain = ToolchainManager::detect_best_toolchain()
                .context("Failed to detect compatible native toolchain")?;

            let planner = BuildPlanner::new(&project, toolchain.as_ref());
            let plans = planner.plan_transitive(target_name)?;
            let root_plan = plans.last().unwrap();

            if !root_plan.output_path.exists() {
                println!(
                    "{}: Target artifact missing, building '{}'...",
                    "glue".cyan().bold(),
                    target_name
                );
                let ninja = NinjaBackend::new(toolchain.as_ref())?;
                let ninja_file = ninja.generate_ninja_file(&plans, planner.build_dir())?;
                ninja.build(&ninja_file)?;
            }
            cache_source_fallbacks(&project, &plans);
            let deployed = planner.deploy_runtime_artifacts(target_name)?;
            for d in deployed {
                println!(
                    "{}: Staged runtime artifact: {}",
                    "glue".cyan().bold(),
                    d.file_name().unwrap().to_string_lossy()
                );
            }

            let mut cmd = Command::new(&root_plan.output_path);
            cmd.args(args);
            let status = cmd.status().context("Failed to execute target binary")?;
            Ok(status.code().unwrap_or(0))
        }
        Commands::Package { target, version } => {
            let curr_dir = std::env::current_dir()?;
            let project = find_and_parse_project(&curr_dir)
                .context("Failed to load Glue project")?;
            let target_name = target
                .as_deref()
                .or(project.default_target.as_deref())
                .ok_or_else(|| anyhow::anyhow!("No target specified and no default target found"))?;
            let target = project
                .get_target(target_name)
                .ok_or_else(|| anyhow::anyhow!("Target '{}' not found", target_name))?;
            if target.kind != TargetKind::StaticLib && target.kind != TargetKind::SharedLib {
                anyhow::bail!("Packaging currently supports static and shared libraries; target '{}' has kind '{:?}'", target_name, target.kind);
            }
            let toolchain = ToolchainManager::detect_best_toolchain()
                .context("Failed to detect compatible native toolchain")?;
            println!("{}: Building package target '{}' using {}", "glue".cyan().bold(), target_name, toolchain.name());
            let planner = BuildPlanner::new(&project, toolchain.as_ref());
            let plans = planner.plan_transitive(target_name)?;
            let root_plan = plans.last().ok_or_else(|| anyhow::anyhow!("Package target has no build plan"))?;
            if has_prebuilt_dependency(&project, target_name, &mut HashSet::new()) {
                anyhow::bail!("Packaging a target that consumes an imported prebuilt package is not supported yet");
            }
            if plans.iter().any(|plan| plan.target.kind != TargetKind::StaticLib && plan.target.kind != TargetKind::SharedLib) {
                anyhow::bail!("This package workflow supports static and shared library dependency closures only");
            }
            let ninja = NinjaBackend::new(toolchain.as_ref())?;
            let ninja_file = ninja.generate_ninja_file(&plans, planner.build_dir())?;
            ninja.build(&ninja_file)?;

            let package_name = root_plan.target.name.clone();
            if !is_safe_package_component(&package_name) || !is_safe_package_component(&version) {
                anyhow::bail!("Package name and version must be safe single path components (letters, numbers, '.', '_' or '-')");
            }
            let namespace = root_plan.target.namespace.clone().unwrap_or_else(|| package_name.clone());
            if !is_safe_package_component(&namespace) {
                anyhow::bail!("Package target namespace must be a safe single path component");
            }
            let toolchain_key = match toolchain.toolchain_type() {
                glue_toolchains::ToolchainType::Msvc => "msvc",
                glue_toolchains::ToolchainType::Clang => "clang",
                glue_toolchains::ToolchainType::Gcc => "gcc",
            };
            let toolchain_fingerprint = PackageManifest::toolchain_fingerprint(&[
                toolchain.compiler_path().to_path_buf(),
                toolchain.linker_path().to_path_buf(),
                toolchain.archiver_path().to_path_buf(),
            ]).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let variant_id = compute_variant_id(&project.root_dir, &plans, &package_name, &version, toolchain.name(), &toolchain_fingerprint)?;
            let variant_dir = curr_dir
                .join("product")
                .join("packages")
                .join(&namespace)
                .join(&package_name)
                .join(&version)
                .join("variants")
                .join(&variant_id);

            let packages_root = curr_dir.join("product").join("packages");
            fs::create_dir_all(&packages_root)?;
            let stage = packages_root.join(format!(".staging-{}-{}", std::process::id(), variant_id));
            if stage.exists() {
                fs::remove_dir_all(&stage)?;
            }
            let has_shared = plans.iter().any(|plan| plan.target.kind == TargetKind::SharedLib);
            fs::create_dir_all(stage.join("lib"))?;
            fs::create_dir_all(stage.join("include"))?;
            if has_shared {
                fs::create_dir_all(stage.join("bin"))?;
            }

            let package_result = (|| -> Result<String> {
                let mut copied_headers = HashSet::new();
                let mut artifact_paths = std::collections::HashMap::new();
                let mut runtime_paths: std::collections::HashMap<String, Vec<PathBuf>> = std::collections::HashMap::new();
                for plan in &plans {
                    if plan.target.kind == TargetKind::SharedLib {
                        if cfg!(windows) {
                            let link_lib = plan.output_path.with_extension("lib");
                            if !link_lib.exists() {
                                anyhow::bail!(
                                    "Shared library import library was not produced: {}. Ensure functions are exported via __declspec(dllexport)",
                                    link_lib.display()
                                );
                            }
                            let link_lib_name = link_lib.file_name().unwrap();
                            let link_rel = PathBuf::from("lib").join(link_lib_name);
                            fs::copy(&link_lib, stage.join(&link_rel))
                                .with_context(|| format!("Failed to stage import library from {}", link_lib.display()))?;
                            artifact_paths.insert(plan.target.qualified_name(), link_rel);

                            let dll_name = plan.output_path.file_name().unwrap();
                            let runtime_rel = PathBuf::from("bin").join(dll_name);
                            fs::copy(&plan.output_path, stage.join(&runtime_rel))
                                .with_context(|| format!("Failed to stage runtime DLL from {}", plan.output_path.display()))?;
                            runtime_paths.insert(plan.target.qualified_name(), vec![runtime_rel]);
                        } else {
                            let so_name = plan.output_path.file_name().unwrap();
                            let link_rel = PathBuf::from("lib").join(so_name);
                            fs::copy(&plan.output_path, stage.join(&link_rel))
                                .with_context(|| format!("Failed to stage shared library interface from {}", plan.output_path.display()))?;
                            artifact_paths.insert(plan.target.qualified_name(), link_rel);

                            let runtime_rel = PathBuf::from("bin").join(so_name);
                            fs::copy(&plan.output_path, stage.join(&runtime_rel))
                                .with_context(|| format!("Failed to stage shared library runtime from {}", plan.output_path.display()))?;
                            runtime_paths.insert(plan.target.qualified_name(), vec![runtime_rel]);
                        }
                    } else {
                        let artifact_name = plan.output_path.file_name().ok_or_else(|| anyhow::anyhow!("Built artifact has no filename"))?;
                        let artifact_rel = PathBuf::from("lib").join(artifact_name);
                        if stage.join(&artifact_rel).exists() {
                            anyhow::bail!("Two package targets produce the same artifact filename: {}", artifact_name.to_string_lossy());
                        }
                        fs::copy(&plan.output_path, stage.join(&artifact_rel))
                            .with_context(|| format!("Failed to stage package artifact from {}", plan.output_path.display()))?;
                        artifact_paths.insert(plan.target.qualified_name(), artifact_rel);
                    }

                    for include in &plan.target.include_dirs_public {
                        let source = if include.is_absolute() { include.clone() } else { curr_dir.join(include) };
                        if source.is_dir() {
                            copy_directory_contents(&source, &stage.join("include"), &stage.join("include"), &mut copied_headers)?;
                        } else {
                            anyhow::bail!("Public include directory does not exist: {}", source.display());
                        }
                    }
                }

                let mut gluefile = String::new();
                let mut exported_targets = Vec::new();
                for plan in &plans {
                    let target_ns = plan.target.namespace.as_deref().unwrap_or(namespace.as_str());
                    let artifact_rel = artifact_paths.get(&plan.target.qualified_name()).unwrap();
                    let kind_decl = match plan.target.kind {
                        TargetKind::SharedLib => "shared_lib",
                        TargetKind::StaticLib => "static_lib",
                        _ => "static_lib",
                    };
                    gluefile.push_str(&format!(
                        "namespace {};\n{}.add({})\n    .prebuilt(\"{}\")",
                        target_ns,
                        kind_decl,
                        plan.target.name,
                        artifact_rel.to_string_lossy().replace('\\', "/")
                    ));
                    if let Some(runtimes) = runtime_paths.get(&plan.target.qualified_name()) {
                        for r in runtimes {
                            gluefile.push_str(&format!(
                                "\n    .runtime(\"{}\")",
                                r.to_string_lossy().replace('\\', "/")
                            ));
                        }
                    }
                    gluefile.push_str("\n    .include.public(\"include\")");
                    if !plan.target.dependencies.is_empty() {
                        let dependencies: Vec<String> = plan.target.dependencies.iter().map(|dependency| {
                            project.get_target(dependency)
                                .map(|target| target.qualified_name())
                                .unwrap_or_else(|| dependency.clone())
                        }).collect();
                        gluefile.push_str(&format!("\n    .depends({})", dependencies.join(", ")));
                    }
                    gluefile.push_str("\n\n");
                    exported_targets.push(plan.target.qualified_name());
                }
                fs::write(stage.join("gluefile"), gluefile)?;
                let artifact_rel = artifact_paths.get(&root_plan.target.qualified_name()).unwrap().clone();
                let root_kind_str = match root_plan.target.kind {
                    TargetKind::SharedLib => "shared_lib",
                    TargetKind::StaticLib => "static_lib",
                    _ => "static_lib",
                };
                let root_runtimes = runtime_paths.get(&root_plan.target.qualified_name()).cloned().unwrap_or_default();
                let manifest = PackageManifest {
                    package: PackageMetadata {
                        name: package_name.clone(),
                        version: version.clone(),
                        source_project: curr_dir.file_name().and_then(|n| n.to_str()).map(str::to_string),
                        description: None,
                        authors: None,
                    },
                    targets: exported_targets,
                    variant: Some(PackageVariant {
                        id: variant_id.clone(),
                        target_os: std::env::consts::OS.to_string(),
                        target_arch: std::env::consts::ARCH.to_string(),
                        toolchain: toolchain_key.to_string(),
                        toolchain_fingerprint,
                        target_name: root_plan.target.name.clone(),
                        target_namespace: Some(namespace.clone()),
                        target_kind: root_kind_str.to_string(),
                        artifact: artifact_rel,
                        runtime_artifacts: root_runtimes,
                        include_dirs: vec![PathBuf::from("include")],
                    }),
                };
                manifest.write_to_dir(&stage)?;
                PackageManifest::write_content_digest(&stage).map_err(|e| anyhow::anyhow!(e.to_string()))
            })();

            let digest = match package_result {
                Ok(digest) => digest,
                Err(err) => {
                    let _ = fs::remove_dir_all(&stage);
                    return Err(err);
                }
            };

            if variant_dir.exists() {
                let existing = PackageManifest::verify_content_digest(&variant_dir)
                    .map_err(|e| anyhow::anyhow!("Existing package variant is invalid: {}", e))?;
                let _ = fs::remove_dir_all(&stage);
                if existing != digest {
                    anyhow::bail!("Package variant '{}' already exists with different contents; refusing to replace it", variant_dir.display());
                }
            } else {
                if let Some(parent) = variant_dir.parent() { fs::create_dir_all(parent)?; }
                fs::rename(&stage, &variant_dir)?;
            }
            let version_root = variant_dir.parent().and_then(Path::parent)
                .ok_or_else(|| anyhow::anyhow!("Invalid package product directory"))?;
            ensure_package_entry(version_root, &package_name, &namespace, &version, &variant_id, &digest)?;
            println!("{}: Package product: {}", "glue".green().bold(), variant_dir.display());
            println!("{}: Content SHA-256: {}", "glue".green().bold(), digest);
            Ok(0)
        }
        Commands::Clean { all } => {
            let curr_dir = std::env::current_dir()?;
            let product_build = curr_dir.join("product").join("build");
            let dot_glue = curr_dir.join(".glue");

            if all {
                let product_dir = curr_dir.join("product");
                let mut cleaned = false;
                if product_dir.exists() {
                    fs::remove_dir_all(&product_dir)?;
                    cleaned = true;
                }
                if dot_glue.exists() {
                    fs::remove_dir_all(&dot_glue)?;
                    cleaned = true;
                }
                if cleaned {
                    println!("{}: Cleaned all products and cache (product/ and .glue/)", "glue".cyan().bold());
                } else {
                    println!("{}: Nothing to clean", "glue".cyan().bold());
                }
            } else {
                let mut cleaned = false;
                if product_build.exists() {
                    fs::remove_dir_all(&product_build)?;
                    cleaned = true;
                }
                let planner_cache = dot_glue.join("planner");
                if planner_cache.exists() {
                    let _ = fs::remove_dir_all(planner_cache);
                    cleaned = true;
                }
                let fp_cache = dot_glue.join("fingerprints");
                if fp_cache.exists() {
                    let _ = fs::remove_dir_all(fp_cache);
                    cleaned = true;
                }
                if cleaned {
                    println!("{}: Cleaned build artifacts at product/build", "glue".cyan().bold());
                } else {
                    println!("{}: Nothing to clean", "glue".cyan().bold());
                }
            }
            Ok(0)
        }
        Commands::Explain { target } => {
            let curr_dir = std::env::current_dir()?;
            let project = find_and_parse_project(&curr_dir)
                .context("Failed to load Glue project")?;
            let target_name = target
                .as_deref()
                .or(project.default_target.as_deref())
                .ok_or_else(|| anyhow::anyhow!("No target specified"))?;
            let t = project
                .get_target(target_name)
                .ok_or_else(|| anyhow::anyhow!("Target '{}' not found", target_name))?;

            println!("\nTarget: {}", t.name.bold());
            println!("Kind:   {:?}", t.kind);
            println!("Sources ({}):", t.sources.len());
            for s in &t.sources {
                println!("  - {}", s.display());
            }
            println!("Include Directories (Public) ({}):", t.include_dirs_public.len());
            for i in &t.include_dirs_public {
                println!("  - {}", i.display());
            }
            println!("Include Directories (Private) ({}):", t.include_dirs_private.len());
            for i in &t.include_dirs_private {
                println!("  - {}", i.display());
            }
            println!("Dependencies ({}):", t.dependencies.len());
            for d in &t.dependencies {
                println!("  - {}", d);
            }
            println!();
            Ok(0)
        }
        Commands::Graph { target } => {
            let curr_dir = std::env::current_dir()?;
            let project = find_and_parse_project(&curr_dir)
                .context("Failed to load Glue project")?;
            let toolchain = ToolchainManager::detect_best_toolchain()
                .context("Failed to detect compatible native toolchain")?;
            let planner = BuildPlanner::new(&project, toolchain.as_ref());

            let targets_to_graph: Vec<&str> = if let Some(ref t) = target {
                vec![t.as_str()]
            } else {
                project.targets.keys().map(|s| s.as_str()).collect()
            };

            println!("digraph GlueBuild {{");
            for t_name in targets_to_graph {
                let plan = planner.plan_target(t_name)?;
                println!("  \"{}\" [shape=box, label=\"Target: {}\"];", t_name, t_name);
                for u in &plan.compile_units {
                    let s = u.source.file_name().unwrap().to_str().unwrap();
                    let o = u.object.file_name().unwrap().to_str().unwrap();
                    println!("  \"{}\" -> \"{}\" [label=\"compile\"];", s, o);
                    println!("  \"{}\" -> \"{}\" [label=\"link\"];", o, t_name);
                }
                for dep in &plan.target.dependencies {
                    println!("  \"{}\" -> \"{}\" [label=\"depends\"];", t_name, dep);
                }
            }
            println!("}}");
            Ok(0)
        }
    }
}

fn is_safe_package_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn has_prebuilt_dependency(project: &glue_model::Project, target_name: &str, visited: &mut HashSet<String>) -> bool {
    if !visited.insert(target_name.to_string()) { return false; }
    let Some(target) = project.get_target(target_name) else { return false; };
    target.prebuilt_artifact.is_some()
        || target.dependencies.iter().any(|dependency| has_prebuilt_dependency(project, dependency, visited))
}

fn ensure_package_entry(
    version_root: &Path,
    package_name: &str,
    namespace: &str,
    version: &str,
    variant_id: &str,
    digest: &str,
) -> Result<()> {
    fs::create_dir_all(version_root)?;

    let entry = version_root.join("gluefile");
    let content = "when imported {\n    return package()\n}\n";
    if !entry.exists() {
        fs::write(&entry, content)?;
    }

    let metadata_path = version_root.join("metadata.json");
    let metadata_content = format!(
        "{{\n  \"namespace\": \"{}\",\n  \"name\": \"{}\",\n  \"version\": \"{}\",\n  \"variants\": [\n    {{\n      \"id\": \"{}\",\n      \"os\": \"{}\",\n      \"arch\": \"{}\",\n      \"digest\": \"{}\"\n    }}\n  ]\n}}\n",
        namespace, package_name, version, variant_id, std::env::consts::OS, std::env::consts::ARCH, digest
    );
    fs::write(&metadata_path, metadata_content)?;
    Ok(())
}

fn cache_source_fallbacks(project: &glue_model::Project, plans: &[TargetPlan]) {
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("glue: could not locate itself to cache source-built packages: {}", error);
            return;
        }
    };
    let mut seen = HashSet::new();
    for plan in plans {
        let Some(source_root) = plan.target.source_root.as_ref() else { continue; };
        let Some(source_target) = plan.target.source_target.as_deref() else { continue; };
        let plan_name = plan.target.qualified_name();
        let is_consumer_entry = project.targets.values()
            .filter(|target| target.source_root.is_none() && target.prebuilt_artifact.is_none())
            .any(|target| target.dependencies.iter().any(|dependency| {
                project.get_target(dependency)
                    .map(|resolved| resolved.qualified_name() == plan_name)
                    .unwrap_or(false)
            }));
        if !is_consumer_entry { continue; }
        let key = (source_root.clone(), source_target.to_string());
        if !seen.insert(key) { continue; }

        // Avoid publishing a partial product when the source target depends on a package
        // binary that is not included in this source build closure.
        let has_external_binary_dependency = plan.target.dependencies.iter().any(|dependency| {
            project.get_target(dependency)
                .map(|target| target.prebuilt_artifact.is_some() || target.source_root.as_ref() != Some(source_root))
                .unwrap_or(true)
        });
        if has_external_binary_dependency { continue; }

        let status = Command::new(&executable)
            .current_dir(source_root)
            .arg("package")
            .arg(source_target)
            .arg("--version")
            .arg("0.1.0")
            .status();
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!("glue: source build succeeded, but caching package '{}' failed with {}", source_target, status),
            Err(error) => eprintln!("glue: source build succeeded, but caching package '{}' failed: {}", source_target, error),
        }
    }
}

fn compute_variant_id(root: &Path, plans: &[TargetPlan], package: &str, version: &str, toolchain: &str, toolchain_fingerprint: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    for value in [package, version, std::env::consts::OS, std::env::consts::ARCH, toolchain, toolchain_fingerprint] {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    for plan in plans {
        let target = &plan.target;
        let target_root = target.source_root.as_deref().unwrap_or(root);
        hasher.update(target.qualified_name().as_bytes());
        hasher.update(format!("{:?}", target.kind).as_bytes());
        for (key, value) in &target.defines {
            hasher.update(key.as_bytes());
            if let Some(value) = value { hasher.update(value.as_bytes()); }
        }
        for flag in &target.compile_flags { hasher.update(flag.as_bytes()); }
        for unit in &plan.compile_units {
            let path = if unit.source.is_absolute() { unit.source.clone() } else { target_root.join(&unit.source) };
            let relative = path.strip_prefix(target_root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            hasher.update(relative.as_bytes());
            if path.is_file() { hasher.update(fs::read(path)?); }
        }
        for include in target.include_dirs_public.iter().chain(target.include_dirs_private.iter()) {
            let path = if include.is_absolute() { include.clone() } else { target_root.join(include) };
            hasher.update(include.to_string_lossy().as_bytes());
            if path.is_dir() { update_directory_fingerprint(&path, &mut hasher)?; }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_ignored_dir(name: &str) -> bool {
    name.starts_with('.') || matches!(name, "product" | "build" | "target" | "obj" | "bin")
}

fn is_header_file(name: &str) -> bool {
    matches!(
        Path::new(name).extension().and_then(|ext| ext.to_str()),
        Some("h" | "hpp" | "hxx" | "hh" | "inc" | "inl")
    )
}

fn update_directory_fingerprint(path: &Path, hasher: &mut Sha256) -> Result<()> {
    let mut entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let child = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            anyhow::bail!("Cannot fingerprint public include symlink: {}", child.display());
        }
        if kind.is_dir() {
            if is_ignored_dir(&name_str) {
                continue;
            }
            hasher.update(name_str.as_bytes());
            update_directory_fingerprint(&child, hasher)?;
        } else if kind.is_file() && is_header_file(&name_str) {
            hasher.update(name_str.as_bytes());
            hasher.update(fs::read(child)?);
        }
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, destination: &Path, package_include_root: &Path, seen: &mut HashSet<PathBuf>) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            anyhow::bail!("Refusing to package a public include symlink: {}", source_path.display());
        } else if kind.is_dir() {
            if is_ignored_dir(&name_str) {
                continue;
            }
            let destination_path = destination.join(&name);
            copy_directory_contents(&source_path, &destination_path, package_include_root, seen)?;
        } else if kind.is_file() && is_header_file(&name_str) {
            let destination_path = destination.join(&name);
            let relative = destination_path.strip_prefix(package_include_root)
                .unwrap_or(&destination_path).to_path_buf();
            if !seen.insert(relative.clone()) {
                anyhow::bail!("Two public include directories contain a colliding path: {}", relative.display());
            }
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}
