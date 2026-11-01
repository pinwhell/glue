use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use glue_backend_ninja::NinjaBackend;
use glue_build::BuildPlanner;
use glue_config::find_and_parse_project;
use glue_toolchains::ToolchainManager;
use std::fs;
use std::path::Path;
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
    },
    /// Build and run an executable target
    Run {
        /// Target to run
        target: Option<String>,
        /// Arguments forwarded to the target
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Remove build artifacts (.glue directory)
    Clean,
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
        Commands::Build { target } => {
            let curr_dir = std::env::current_dir()?;
            let project = find_and_parse_project(&curr_dir)
                .context("Failed to load Glue project")?;
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

            let mut cmd = Command::new(&root_plan.output_path);
            cmd.args(args);
            let status = cmd.status().context("Failed to execute target binary")?;
            Ok(status.code().unwrap_or(0))
        }
        Commands::Clean => {
            let build_dir = Path::new(".glue");
            if build_dir.exists() {
                fs::remove_dir_all(build_dir)?;
                println!("{}: Cleaned build artifacts at .glue", "glue".cyan().bold());
            } else {
                println!("{}: Nothing to clean", "glue".cyan().bold());
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
