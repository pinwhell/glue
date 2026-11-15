use glue_model::{Project, Target, TargetKind};
use glue_packages::PackageManifest;
use glue_toolchains::{ToolchainManager, ToolchainType};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Lexer error at line {line}, col {col}: {message}")]
    LexError {
        line: usize,
        col: usize,
        message: String,
    },
    #[error("Parse error at line {line}, col {col}: {message}")]
    ParseError {
        line: usize,
        col: usize,
        message: String,
    },
    #[error("Semantic error: {0}")]
    SemanticError(String),
    #[error("IO error reading configuration: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    StringLit(String),
    Number(i64),
    Dot,
    Slash,
    LParen,
    RParen,
    LBrace,
    RBrace,
    SemiColon,
    ColonColon,
    Comma,
    At,
    Import,
    From,
    Version,
    Namespace,
    When,
    Imported,
    Return,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer<'a> {
    _input: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            _input: input,
            chars: input.chars().enumerate().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|&(_, c)| c)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(&(_, c)) = self.chars.get(self.pos) {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, ConfigError> {
        let mut tokens = Vec::new();

        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
                continue;
            }

            let start_line = self.line;
            let start_col = self.col;

            // Single line comments starting with // or #
            if c == '#' || (c == '/' && self.chars.get(self.pos + 1).map(|&(_, next)| next) == Some('/')) {
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            match c {
                '.' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::Dot, line: start_line, col: start_col });
                }
                '/' | '\\' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::Slash, line: start_line, col: start_col });
                }
                ';' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::SemiColon, line: start_line, col: start_col });
                }
                '{' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::LBrace, line: start_line, col: start_col });
                }
                '}' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::RBrace, line: start_line, col: start_col });
                }
                ':' => {
                    self.advance();
                    if self.peek() == Some(':') {
                        self.advance();
                        tokens.push(Token { kind: TokenKind::ColonColon, line: start_line, col: start_col });
                    }
                }
                '(' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::LParen, line: start_line, col: start_col });
                }
                ')' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::RParen, line: start_line, col: start_col });
                }
                ',' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::Comma, line: start_line, col: start_col });
                }
                '@' => {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::At, line: start_line, col: start_col });
                }
                '"' | '\'' => {
                    let quote = c;
                    self.advance();
                    let mut s = String::new();
                    let mut closed = false;
                    while let Some(ch) = self.advance() {
                        if ch == quote {
                            closed = true;
                            break;
                        }
                        if ch == '\\' {
                            if let Some(escaped) = self.advance() {
                                s.push(escaped);
                            }
                        } else {
                            s.push(ch);
                        }
                    }
                    if !closed {
                        return Err(ConfigError::LexError {
                            line: start_line,
                            col: start_col,
                            message: "Unterminated string literal".into(),
                        });
                    }
                    tokens.push(Token {
                        kind: TokenKind::StringLit(s),
                        line: start_line,
                        col: start_col,
                    });
                }
                _ if c.is_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    while let Some(ch) = self.peek() {
                        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                            ident.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let kind = match ident.as_str() {
                        "import" => TokenKind::Import,
                        "from" => TokenKind::From,
                        "version" => TokenKind::Version,
                        "namespace" => TokenKind::Namespace,
                        "when" => TokenKind::When,
                        "imported" => TokenKind::Imported,
                        "return" => TokenKind::Return,
                        _ => TokenKind::Ident(ident),
                    };
                    tokens.push(Token {
                        kind,
                        line: start_line,
                        col: start_col,
                    });
                }
                _ if c.is_ascii_digit() => {
                    let mut num_str = String::new();
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_digit() {
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let num: i64 = num_str.parse().unwrap_or(0);
                    tokens.push(Token {
                        kind: TokenKind::Number(num),
                        line: start_line,
                        col: start_col,
                    });
                }
                _ => {
                    self.advance();
                    return Err(ConfigError::LexError {
                        line: start_line,
                        col: start_col,
                        message: format!("Unexpected character: '{}'", c),
                    });
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            col: self.col,
        });

        Ok(tokens)
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    import_stack: Vec<PathBuf>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0, import_stack: Vec::new() }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.cursor];
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
        tok
    }

    fn expect_ident(&mut self) -> Result<String, ConfigError> {
        let tok = self.peek().clone();
        if let TokenKind::Ident(s) = tok.kind {
            self.advance();
            Ok(s)
        } else {
            Err(ConfigError::ParseError {
                line: tok.line,
                col: tok.col,
                message: format!("Expected identifier, found {:?}", tok.kind),
            })
        }
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<(), ConfigError> {
        let tok = self.peek().clone();
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(expected) {
            self.advance();
            Ok(())
        } else {
            Err(ConfigError::ParseError {
                line: tok.line,
                col: tok.col,
                message: format!("Expected {:?}, found {:?}", expected, tok.kind),
            })
        }
    }

    fn parse_string_or_ident(&mut self) -> Result<String, ConfigError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::StringLit(s) | TokenKind::Ident(s) => {
                self.advance();
                Ok(s)
            }
            _ => Err(ConfigError::ParseError {
                line: tok.line,
                col: tok.col,
                message: format!("Expected string or identifier, found {:?}", tok.kind),
            }),
        }
    }

    fn parse_import_source(&mut self) -> Result<String, ConfigError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(s)
            }
            TokenKind::Ident(mut s) => {
                let start_line = tok.line;
                self.advance();
                while self.peek().line == start_line
                    && matches!(
                        self.peek().kind,
                        TokenKind::At
                            | TokenKind::Slash
                            | TokenKind::Dot
                            | TokenKind::ColonColon
                            | TokenKind::Ident(_)
                            | TokenKind::Number(_)
                    )
                {
                    match &self.peek().kind {
                        TokenKind::At => {
                            self.advance();
                            s.push('@');
                        }
                        TokenKind::Slash => {
                            self.advance();
                            s.push('/');
                        }
                        TokenKind::Dot => {
                            self.advance();
                            s.push('.');
                        }
                        TokenKind::ColonColon => {
                            self.advance();
                            s.push_str("::");
                        }
                        TokenKind::Ident(part) => {
                            let part = part.clone();
                            self.advance();
                            s.push_str(&part);
                        }
                        TokenKind::Number(n) => {
                            let n_str = n.to_string();
                            self.advance();
                            s.push_str(&n_str);
                        }
                        _ => break,
                    }
                }
                Ok(s)
            }
            _ => Err(ConfigError::ParseError {
                line: tok.line,
                col: tok.col,
                message: format!(
                    "Expected import source path or package identifier (e.g. \"../path\" or github@user/repo), found {:?}",
                    tok.kind
                ),
            }),
        }
    }

    fn parse_path_or_ident(&mut self) -> Result<String, ConfigError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(s)
            }
            TokenKind::Ident(mut s) => {
                self.advance();
                while matches!(
                    self.peek().kind,
                    TokenKind::Dot | TokenKind::Slash | TokenKind::ColonColon | TokenKind::Ident(_)
                ) {
                    match &self.peek().kind {
                        TokenKind::Dot => {
                            self.advance();
                            s.push('.');
                        }
                        TokenKind::Slash => {
                            self.advance();
                            s.push('/');
                        }
                        TokenKind::ColonColon => {
                            self.advance();
                            s.push_str("::");
                        }
                        TokenKind::Ident(part) => {
                            let part = part.clone();
                            self.advance();
                            s.push_str(&part);
                        }
                        _ => break,
                    }
                }
                Ok(s)
            }
            _ => Err(ConfigError::ParseError {
                line: tok.line,
                col: tok.col,
                message: format!("Expected string, identifier or path, found {:?}", tok.kind),
            }),
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<String>, ConfigError> {
        self.expect(&TokenKind::LParen)?;
        let mut args = Vec::new();
        while self.peek().kind != TokenKind::RParen && self.peek().kind != TokenKind::Eof {
            args.push(self.parse_path_or_ident()?);
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RParen)?;
        Ok(args)
    }

    pub fn parse_project(&mut self, project_name: &str, root_dir: &Path) -> Result<Project, ConfigError> {
        let mut project = Project::new(project_name, root_dir.to_path_buf());
        let mut current_namespace: Option<String> = None;
        let mut namespace_stack: Vec<Option<String>> = Vec::new();

        while self.peek().kind != TokenKind::Eof {
            let tok = self.peek().clone();

            match tok.kind {
                TokenKind::SemiColon => {
                    self.advance();
                }
                TokenKind::When => {
                    self.parse_import_dispatch()?;
                }
                TokenKind::Namespace => {
                    self.advance();
                    let ns_name = self.parse_string_or_ident()?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                        current_namespace = Some(ns_name);
                    } else if self.peek().kind == TokenKind::LBrace {
                        self.advance();
                        namespace_stack.push(current_namespace.clone());
                        current_namespace = Some(ns_name);
                    } else {
                        return Err(ConfigError::ParseError {
                            line: tok.line,
                            col: tok.col,
                            message: "Expected ';' or '{' after namespace declaration".to_string(),
                        });
                    }
                }
                TokenKind::RBrace => {
                    self.advance();
                    if let Some(prev) = namespace_stack.pop() {
                        current_namespace = prev;
                    } else {
                        current_namespace = None;
                    }
                }
                TokenKind::Import => {
                    self.advance();
                    let package_name = self.parse_string_or_ident()?;
                    self.expect(&TokenKind::From)?;
                    let source = self.parse_import_source()?;
                    let version_req = if self.peek().kind == TokenKind::Version {
                        self.advance();
                        Some(self.parse_string_or_ident()?)
                    } else { None };
                    let import_root = resolve_package_source(root_dir, &source)?;
                    self.merge_import(&mut project, &package_name, &import_root, version_req.as_deref())?;
                    project.imported_packages.push(glue_model::ImportedPackageRecord {
                        alias: package_name,
                        source,
                        resolved_path: import_root,
                        version_req,
                    });
                }
                TokenKind::Ident(ref target_type) => {
                    let kind = match target_type.as_str() {
                        "exec" => TargetKind::Exec,
                        "static_lib" => TargetKind::StaticLib,
                        "shared_lib" => TargetKind::SharedLib,
                        "object_lib" => TargetKind::ObjectLib,
                        "interface_lib" => TargetKind::InterfaceLib,
                        "custom" => TargetKind::Custom,
                        other => {
                            return Err(ConfigError::ParseError {
                                line: tok.line,
                                col: tok.col,
                                message: format!("Unknown target declaration or statement: '{}'", other),
                            });
                        }
                    };
                    self.advance(); // consume target kind
                    self.expect(&TokenKind::Dot)?;
                    let method = self.expect_ident()?;
                    if method != "add" {
                        return Err(ConfigError::ParseError {
                            line: tok.line,
                            col: tok.col,
                            message: format!("Expected '.add', found '.{}'", method),
                        });
                    }

                    self.expect(&TokenKind::LParen)?;
                    let target_name = self.parse_string_or_ident()?;
                    self.expect(&TokenKind::RParen)?;

                    let mut target = Target::new(&target_name, kind);
                    if let Some(ref ns) = current_namespace {
                        target = target.with_namespace(ns);
                    }

                    // Parse method chain: .sources(...), .include.public(...), .depends(...), etc.
                    while self.peek().kind == TokenKind::Dot {
                        self.advance(); // consume dot
                        let prop = self.expect_ident()?;

                        match prop.as_str() {
                            "sources" => {
                                if self.peek().kind == TokenKind::Dot {
                                    self.advance();
                                    let sub = self.expect_ident()?;
                                    if sub == "regex" {
                                        let patterns = self.parse_arguments()?;
                                        for p in patterns {
                                            target.sources.push(PathBuf::from(format!("regex:{}", p)));
                                        }
                                    } else {
                                        return Err(ConfigError::ParseError {
                                            line: tok.line,
                                            col: tok.col,
                                            message: format!("Unknown sources modifier: '.{}'", sub),
                                        });
                                    }
                                } else {
                                    let srcs = self.parse_arguments()?;
                                    for s in srcs {
                                        target.sources.push(PathBuf::from(s));
                                    }
                                }
                            }
                            "depends" => {
                                let deps = self.parse_arguments()?;
                                for d in deps {
                                    target.dependencies.push(d);
                                }
                            }
                            "include" => {
                                self.expect(&TokenKind::Dot)?;
                                let scope = self.expect_ident()?;
                                let dirs = self.parse_arguments()?;
                                for d in dirs {
                                    if scope == "public" {
                                        target.include_dirs_public.push(PathBuf::from(d));
                                    } else {
                                        target.include_dirs_private.push(PathBuf::from(d));
                                    }
                                }
                            }
                            "define" | "defines" => {
                                let defs = self.parse_arguments()?;
                                if let Some(key) = defs.first() {
                                    if let Some((k, v)) = key.split_once('=') {
                                        target.defines.push((k.to_string(), Some(v.to_string())));
                                    } else {
                                        let val = defs.get(1).cloned();
                                        target.defines.push((key.clone(), val));
                                    }
                                }
                            }
                            "flags" => {
                                let flags = self.parse_arguments()?;
                                for f in flags {
                                    target.compile_flags.push(f);
                                }
                            }
                            "output_name" => {
                                let out = self.parse_arguments()?;
                                if let Some(name) = out.into_iter().next() {
                                    target.output_name = Some(name);
                                }
                            }
                            "prebuilt" => {
                                let values = self.parse_arguments()?;
                                let artifact = values.first().ok_or_else(|| ConfigError::ParseError {
                                    line: tok.line,
                                    col: tok.col,
                                    message: "'.prebuilt' requires an artifact path".to_string(),
                                })?;
                                target.prebuilt_artifact = Some(PathBuf::from(artifact));
                            }
                            "runtime" => {
                                let values = self.parse_arguments()?;
                                for v in values {
                                    target.runtime_artifacts.push(PathBuf::from(v));
                                }
                            }
                            unknown => {
                                return Err(ConfigError::ParseError {
                                    line: tok.line,
                                    col: tok.col,
                                    message: format!("Unknown target method: '{}'", unknown),
                                });
                            }
                        }
                    }

                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }

                    let qualified = target.qualified_name();
                    if project.targets.contains_key(&qualified) {
                        return Err(ConfigError::SemanticError(format!(
                            "Duplicate target declared: '{}'",
                            qualified
                        )));
                    }

                    project.add_target(target);
                }
                _ => {
                    return Err(ConfigError::ParseError {
                        line: tok.line,
                        col: tok.col,
                        message: format!("Unexpected token: {:?}", tok.kind),
                    });
                }
            }
        }

        // Semantic Validation: check dependencies exist
        for target in project.targets.values() {
            for dep in &target.dependencies {
                if !project.targets.contains_key(dep) {
                    return Err(ConfigError::SemanticError(format!(
                        "Target '{}' depends on unknown target '{}'",
                        target.name, dep
                    )));
                }
            }
        }

        Ok(project)
    }

    fn parse_import_dispatch(&mut self) -> Result<(), ConfigError> {
        let start = self.peek().clone();
        self.advance(); // when
        self.expect(&TokenKind::Imported)?;
        self.expect(&TokenKind::LBrace)?;
        let return_token = self.peek().clone();
        self.expect(&TokenKind::Return)?;
        let function = self.expect_ident()?;
        if function != "package" {
            return Err(ConfigError::ParseError {
                line: return_token.line,
                col: return_token.col,
                message: "The only supported import dispatch is 'return package()'".to_string(),
            });
        }
        self.expect(&TokenKind::LParen)?;
        self.expect(&TokenKind::RParen)?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        self.expect(&TokenKind::RBrace).map_err(|_| ConfigError::ParseError {
            line: start.line,
            col: start.col,
            message: "Expected '}' after when imported dispatch".to_string(),
        })?;
        Ok(())
    }

    fn merge_import(&mut self, project: &mut Project, alias: &str, source: &Path, version_req: Option<&str>) -> Result<(), ConfigError> {
        let source = strip_unc(source.canonicalize().map_err(|e| ConfigError::SemanticError(format!(
            "Cannot resolve imported package '{}' at '{}': {}", alias, source.display(), e
        )))?);
        if self.import_stack.contains(&source) {
            return Err(ConfigError::SemanticError(format!(
                "Circular package import detected at '{}'", source.display()
            )));
        }

        let toolchain = ToolchainManager::detect_best_toolchain().map_err(|e| {
            ConfigError::SemanticError(format!("Cannot select a toolchain while resolving package '{}': {}", alias, e))
        })?;
        let toolchain_key = match toolchain.toolchain_type() {
            ToolchainType::Msvc => "msvc",
            ToolchainType::Clang => "clang",
            ToolchainType::Gcc => "gcc",
        };
        let toolchain_fingerprint = PackageManifest::toolchain_fingerprint(&[
            toolchain.compiler_path().to_path_buf(),
            toolchain.linker_path().to_path_buf(),
            toolchain.archiver_path().to_path_buf(),
        ]).map_err(|e| ConfigError::SemanticError(format!("Cannot fingerprint toolchain for package '{}': {}", alias, e)))?;

        if let Some((variant_dir, manifest)) = find_compatible_product(&source, toolchain_key, &toolchain_fingerprint, version_req)? {
            PackageManifest::verify_content_digest(&variant_dir)
                .map_err(|e| ConfigError::SemanticError(format!("Imported package '{}' failed integrity verification: {}", alias, e)))?;
            let gluefile = variant_dir.join("gluefile");
            let imported = parse_file_with_stack(&gluefile, self.import_stack.clone())?;
            validate_prebuilt_product(&imported, &variant_dir, alias)?;
            merge_imported_targets(project, alias, &variant_dir, imported.targets.into_values())?;
            if manifest.variant.is_none() {
                return Err(ConfigError::SemanticError(format!("Package '{}' has no target variant metadata", alias)));
            }
            return Ok(());
        }

        if source.join("variants").is_dir() && source.join("gluefile").is_file() {
            return Err(ConfigError::SemanticError(format!(
                "Package product '{}' has no variant compatible with {}-{} and toolchain {}; import its source project for a source build or produce the missing variant",
                source.display(), std::env::consts::OS, std::env::consts::ARCH, toolchain_key
            )));
        }
        // No compatible prebuilt product exists. Fall back to the source project and compile its
        // targets as part of the consumer's graph (Binary-First, Source-Capable). The import dispatch
        // block is parsed as a declarative no-op in this source-build context, so it cannot recurse.
        let config = find_config_file(&source).ok_or_else(|| {
            if let Some(version) = version_req {
                ConfigError::SemanticError(format!(
                    "No local product for package '{}' at version '{}' matches this target/toolchain, and no source project found at '{}'",
                    alias, version, source.display()
                ))
            } else {
                ConfigError::SemanticError(format!(
                    "Package '{}' has no compatible product and no local Glue source project at '{}'",
                    alias, source.display()
                ))
            }
        })?;
        let mut stack = self.import_stack.clone();
        stack.push(source.clone());
        let imported = parse_file_with_stack(&config, stack)?;
        merge_imported_targets(project, alias, &source, imported.targets.into_values())
    }
}

fn find_config_file(dir: &Path) -> Option<PathBuf> {
    ["here.glue", "gluefile", "glue.glue", "Gluefile"]
        .iter().map(|name| dir.join(name)).find(|path| path.is_file())
}

fn collect_manifests(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> Result<(), ConfigError> {
    let entries = fs::read_dir(current).map_err(|e| ConfigError::SemanticError(format!(
        "Cannot inspect package products at '{}': {}", current.display(), e
    )))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()).map(|n| n == ".git" || n == ".glue" || n.starts_with(".staging-")).unwrap_or(false) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            collect_manifests(root, &path, output)?;
        } else if path.file_name().and_then(|n| n.to_str()) == Some("package.toml") && path.starts_with(root) {
            output.push(path);
        }
    }
    Ok(())
}

fn validate_prebuilt_product(project: &Project, product_root: &Path, alias: &str) -> Result<(), ConfigError> {
    let root = product_root.canonicalize()?;
    let mut visited = HashSet::new();
    for target in project.targets.values() {
        if !visited.insert(target.qualified_name()) { continue; }
        let artifact = target.prebuilt_artifact.as_ref().ok_or_else(|| ConfigError::SemanticError(format!(
            "Package product '{}' exports target '{}' without a prebuilt artifact", alias, target.qualified_name()
        )))?;
        let artifact = if artifact.is_absolute() { artifact.clone() } else { product_root.join(artifact) };
        let artifact = artifact.canonicalize().map_err(|e| ConfigError::SemanticError(format!(
            "Package '{}' artifact is missing or unreadable: {} ({})", alias, artifact.display(), e
        )))?;
        if !artifact.starts_with(&root) || !artifact.is_file() {
            return Err(ConfigError::SemanticError(format!(
                "Package '{}' artifact escapes its product directory: {}", alias, artifact.display()
            )));
        }
        for runtime in &target.runtime_artifacts {
            let runtime_path = if runtime.is_absolute() { runtime.clone() } else { product_root.join(runtime) };
            let canonical = runtime_path.canonicalize().map_err(|e| ConfigError::SemanticError(format!(
                "Package '{}' runtime artifact is missing or unreadable: {} ({})", alias, runtime_path.display(), e
            )))?;
            if !canonical.starts_with(&root) || !canonical.is_file() {
                return Err(ConfigError::SemanticError(format!(
                    "Package '{}' runtime artifact escapes its product directory: {}", alias, canonical.display()
                )));
            }
        }
        for include in &target.include_dirs_public {
            let include = if include.is_absolute() { include.clone() } else { product_root.join(include) };
            let canonical = include.canonicalize().map_err(|e| ConfigError::SemanticError(format!(
                "Package '{}' public include directory is missing: {} ({})", alias, include.display(), e
            )))?;
            if !canonical.starts_with(&root) || !canonical.is_dir() {
                return Err(ConfigError::SemanticError(format!(
                    "Package '{}' public include directory escapes its product directory: {}", alias, canonical.display()
                )));
            }
        }
    }
    Ok(())
}

fn find_compatible_product(source: &Path, toolchain_key: &str, toolchain_fingerprint: &str, version_req: Option<&str>) -> Result<Option<(PathBuf, PackageManifest)>, ConfigError> {
    let direct_product = source.join("package.toml").is_file() || source.join("variants").is_dir();
    let packages_root = if direct_product {
        source.to_path_buf()
    } else {
        source.join("product").join("packages")
    };
    if !packages_root.is_dir() {
        return Ok(None);
    }
    let mut manifests = Vec::new();
    collect_manifests(&packages_root, &packages_root, &mut manifests)?;
    let mut compatible = Vec::new();
    for path in manifests {
        let manifest = PackageManifest::load_from_dir(path.parent().unwrap()).map_err(|e| ConfigError::SemanticError(format!(
            "Invalid package manifest '{}': {}", path.display(), e
        )))?;
        if version_req.map(|version| manifest.package.version != version).unwrap_or(false) { continue; }
        let Some(variant) = manifest.variant.as_ref() else { continue; };
        if !direct_product {
            let expected_namespace = source.file_name().and_then(|name| name.to_str());
            let project_matches = manifest.package.source_project.as_deref() == expected_namespace;
            let ns_matches = variant.target_namespace.as_deref() == expected_namespace;
            let name_matches = expected_namespace.map(|exp| manifest.package.name == exp).unwrap_or(false);
            if !project_matches && !ns_matches && !name_matches { continue; }
        }
        if variant.target_os == std::env::consts::OS
            && variant.target_arch == std::env::consts::ARCH
            && variant.toolchain == toolchain_key
            && variant.toolchain_fingerprint == toolchain_fingerprint
        {
            compatible.push((path.parent().unwrap().to_path_buf(), manifest));
        }
    }
    compatible.sort_by(|a, b| a.0.cmp(&b.0));
    match compatible.len() {
        0 => Ok(None),
        1 => Ok(compatible.pop()),
        _ => Err(ConfigError::SemanticError(format!(
            "More than one compatible product variant was found at '{}'; import an explicit package version or variant", packages_root.display()
        ))),
    }
}

fn merge_imported_targets<I>(
    project: &mut Project,
    alias: &str,
    package_root: &Path,
    targets: I,
) -> Result<(), ConfigError>
where I: IntoIterator<Item = Target> {
    let mut merged = HashSet::new();
    let previous_default = project.default_target.clone();
    for mut target in targets {
        let source_name = target.qualified_name();
        if !merged.insert(source_name.clone()) {
            continue;
        }
        if target.prebuilt_artifact.is_none() {
            target.source_target = Some(source_name.clone());
        }
        let old_namespace = target.namespace.clone();
        target.namespace = Some(alias.to_string());
        for dependency in &mut target.dependencies {
            if let Some(old) = old_namespace.as_deref() {
                if let Some(rest) = dependency.strip_prefix(&format!("{}::", old)) {
                    *dependency = format!("{}::{}", alias, rest);
                }
            }
        }
        for include in target.include_dirs_public.iter_mut().chain(target.include_dirs_private.iter_mut()) {
            if !include.is_absolute() { *include = package_root.join(&*include); }
        }
        if let Some(artifact) = target.prebuilt_artifact.as_mut() {
            if !artifact.is_absolute() { *artifact = package_root.join(&*artifact); }
        } else {
            target.source_root = Some(package_root.to_path_buf());
        }
        for runtime in &mut target.runtime_artifacts {
            if !runtime.is_absolute() { *runtime = package_root.join(&*runtime); }
        }
        let qualified = target.qualified_name();
        if project.targets.contains_key(&qualified) {
            return Err(ConfigError::SemanticError(format!("Imported package '{}' collides with target '{}'", alias, qualified)));
        }
        project.add_target(target);
    }
    project.default_target = previous_default;
    Ok(())
}

pub fn parse_file(path: &Path) -> Result<Project, ConfigError> {
    parse_file_with_stack(path, Vec::new())
}

fn parse_file_with_stack(path: &Path, import_stack: Vec<PathBuf>) -> Result<Project, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut lexer = Lexer::new(&content);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.import_stack = import_stack;
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("glue_project");
    parser.parse_project(name, parent)
}

pub fn find_and_parse_project(dir: &Path) -> Result<Project, ConfigError> {
    if let Some(candidate) = find_config_file(dir) {
        let canonical_root = strip_unc(dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()));
        return parse_file_with_stack(&candidate, vec![canonical_root]);
    }

    Err(ConfigError::SemanticError(format!(
        "No Glue configuration file found in '{}' (looked for here.glue, gluefile)",
        dir.display()
    )))
}

fn strip_unc(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}

fn resolve_package_source(root_dir: &Path, source: &str) -> Result<PathBuf, ConfigError> {
    if let Some(slug) = source.strip_prefix("github@") {
        let slug = slug.trim();
        let cache_dir = root_dir
            .join(".glue")
            .join("cache")
            .join("sources")
            .join("github.com")
            .join(slug);

        if cache_dir.exists()
            && (cache_dir.join("gluefile").exists()
                || cache_dir.join("here.glue").exists()
                || cache_dir.join("product").exists())
        {
            return Ok(cache_dir);
        }

        // Try git clone
        if let Some(parent) = cache_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let clone_url = format!("https://github.com/{}.git", slug);
        let clone_status = std::process::Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(&clone_url)
            .arg(&cache_dir)
            .status();

        if let Ok(status) = clone_status {
            if status.success() && cache_dir.exists() {
                return Ok(cache_dir);
            }
        }

        // Fallback: If git clone failed or offline, check common local development directories
        if let Some((_owner, repo_name)) = slug.split_once('/') {
            let mut candidates = vec![
                PathBuf::from(r"C:\Users\User\Documents\GitHub").join(repo_name),
            ];
            if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                candidates.push(PathBuf::from(user_profile).join("Documents").join("GitHub").join(repo_name));
            }
            for candidate in &candidates {
                if candidate.is_dir()
                    && (candidate.join("gluefile").exists()
                        || candidate.join("here.glue").exists()
                        || candidate.join("product").exists())
                {
                    let _ = copy_directory_tree(candidate, &cache_dir);
                    if cache_dir.exists() {
                        return Ok(cache_dir);
                    }
                    return Ok(candidate.clone());
                }
            }
        }

        return Err(ConfigError::SemanticError(format!(
            "Failed to acquire remote package '{}' from GitHub: clone failed and no local fallback found",
            source
        )));
    }

    let import_path = PathBuf::from(source);
    if import_path.is_absolute() {
        Ok(import_path)
    } else {
        Ok(root_dir.join(import_path))
    }
}

fn copy_directory_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if entry.file_name() == ".git" {
            continue;
        }
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory_tree(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
