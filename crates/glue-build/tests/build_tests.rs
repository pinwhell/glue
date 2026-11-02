use glue_build::{BuildError, BuildPlanner};
use glue_model::{Project, Target, TargetKind};
use glue_toolchains::ToolchainManager;
use std::path::PathBuf;

#[test]
fn test_detect_circular_dependency() {
    let mut proj = Project::new("test", PathBuf::from("."));

    let mut t1 = Target::new("a", TargetKind::Exec);
    t1.dependencies.push("b".to_string());

    let mut t2 = Target::new("b", TargetKind::StaticLib);
    t2.dependencies.push("a".to_string());

    proj.add_target(t1);
    proj.add_target(t2);

    let toolchain = ToolchainManager::detect_best_toolchain().expect("toolchain required");
    let planner = BuildPlanner::new(&proj, toolchain.as_ref());

    let res = planner.plan_target("a");
    match res {
        Err(BuildError::CircularDependency(t)) => assert_eq!(t, "a"),
        other => panic!("Expected circular dependency error, got {:?}", other),
    }
}

#[test]
fn test_expand_glob_pattern_sources() {
    let temp_dir = std::env::temp_dir().join(format!("glue_test_glob_{}", std::process::id()));
    let src_dir = temp_dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("one.c"), "int one = 1;").unwrap();
    std::fs::write(src_dir.join("two.c"), "int two = 2;").unwrap();

    let mut proj = Project::new("test_glob", temp_dir.clone());
    let mut target = Target::new("app", TargetKind::Exec);
    target.sources.push(PathBuf::from("src/*.c"));
    proj.add_target(target);

    let toolchain = ToolchainManager::detect_best_toolchain().expect("toolchain required");
    let planner = BuildPlanner::new(&proj, toolchain.as_ref());
    let plan = planner.plan_target("app").expect("planning failed");

    assert_eq!(plan.compile_units.len(), 2);
    let src_names: Vec<String> = plan
        .compile_units
        .iter()
        .map(|u| u.source.file_name().unwrap().to_str().unwrap().to_string())
        .collect();
    assert_eq!(src_names, vec!["one.c", "two.c"]);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
