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
