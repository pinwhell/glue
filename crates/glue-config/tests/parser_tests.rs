use glue_config::{Lexer, Parser};
use glue_model::TargetKind;
use std::path::Path;

#[test]
fn test_parse_minimal_exec() {
    let input = r#"exec.add(hello).sources("hello.c")"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexing failed");
    let mut parser = Parser::new(tokens);
    let project = parser
        .parse_project("test_proj", Path::new("."))
        .expect("parsing failed");

    assert_eq!(project.targets.len(), 1);
    let target = project.get_target("hello").expect("target hello missing");
    assert_eq!(target.kind, TargetKind::Exec);
    assert_eq!(target.sources.len(), 1);
    assert_eq!(target.sources[0].to_str().unwrap(), "hello.c");
}

#[test]
fn test_parse_library_and_consumer() {
    let input = r#"
static_lib.add(core)
    .sources("src/core.c")
    .include.public("include")

exec.add(app)
    .sources("src/main.c")
    .depends(core)
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexing failed");
    let mut parser = Parser::new(tokens);
    let project = parser
        .parse_project("test_proj", Path::new("."))
        .expect("parsing failed");

    assert_eq!(project.targets.len(), 2);
    let core = project.get_target("core").unwrap();
    assert_eq!(core.kind, TargetKind::StaticLib);
    assert_eq!(core.include_dirs_public.len(), 1);

    let app = project.get_target("app").unwrap();
    assert_eq!(app.kind, TargetKind::Exec);
    assert_eq!(app.dependencies, vec!["core"]);
}

#[test]
fn test_parse_missing_dependency_fails_semantically() {
    let input = r#"
exec.add(app)
    .sources("src/main.c")
    .depends(non_existent)
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexing failed");
    let mut parser = Parser::new(tokens);
    let res = parser.parse_project("test_proj", Path::new("."));
    assert!(res.is_err(), "Expected semantic failure for missing dependency");
}

#[test]
fn test_parse_namespaces_file_level() {
    let input = r#"
namespace myns;

exec.add(app)
    .sources("src/main.c")
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexing failed");
    let mut parser = Parser::new(tokens);
    let project = parser
        .parse_project("test_proj", Path::new("."))
        .expect("parsing failed");

    let target = project.get_target("myns::app").expect("myns::app missing");
    assert_eq!(target.namespace.as_deref(), Some("myns"));
    let target_short = project.get_target("app").expect("app short missing");
    assert_eq!(target_short.name, "app");
}

#[test]
fn test_parse_namespaces_block_level() {
    let input = r#"
namespace core {
    static_lib.add(core)
        .sources("src/core.c")
}

namespace app {
    exec.add(app)
        .sources("src/main.c")
        .depends(core::core)
}
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexing failed");
    let mut parser = Parser::new(tokens);
    let project = parser
        .parse_project("test_proj", Path::new("."))
        .expect("parsing failed");

    let core = project.get_target("core::core").expect("core::core missing");
    assert_eq!(core.namespace.as_deref(), Some("core"));

    let app = project.get_target("app::app").expect("app::app missing");
    assert_eq!(app.namespace.as_deref(), Some("app"));
    assert_eq!(app.dependencies, vec!["core::core"]);
}
