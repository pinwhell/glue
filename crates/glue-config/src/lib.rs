use glue_model::{Project, Target, TargetKind};
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
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
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
                    let _package_name = self.parse_string_or_ident()?;
                    self.expect(&TokenKind::From)?;
                    let _source = self.parse_string_or_ident()?;
                    if self.peek().kind == TokenKind::Version {
                        self.advance();
                        let _version_req = self.parse_string_or_ident()?;
                    }
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
}

pub fn parse_file(path: &Path) -> Result<Project, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut lexer = Lexer::new(&content);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("glue_project");
    parser.parse_project(name, parent)
}

pub fn find_and_parse_project(dir: &Path) -> Result<Project, ConfigError> {
    let candidates = [
        dir.join("here.glue"),
        dir.join("gluefile"),
        dir.join("glue.glue"),
        dir.join("Gluefile"),
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            return parse_file(candidate);
        }
    }

    Err(ConfigError::SemanticError(format!(
        "No Glue configuration file found in '{}' (looked for here.glue, gluefile)",
        dir.display()
    )))
}
