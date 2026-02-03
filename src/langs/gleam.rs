use std::fs;

use tree_sitter::{Node, Parser, Tree};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[".git", "build", "fixtures"];
const GLEAM_PATTERNS: &[&str] = &["**/*.gleam"];
const GLEAM_TOML_PATTERNS: &[&str] = &["**/gleam.toml"];

struct GleamTomlRule {
    check: fn(toml: &toml::Value, path: &str) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const GLEAM_TOML_RULES: &[GleamTomlRule] = &[GleamTomlRule {
    check: check_exact_versions,
    hint: None,
    name: "exact-versions",
}];

struct GleamRule {
    check: fn(source: &str, tree: &Tree) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const GLEAM_RULES: &[GleamRule] = &[
    GleamRule {
        check: check_no_ignored_error,
        hint: Some("handle or propagate the error instead of ignoring it"),
        name: "no-ignored-error",
    },
    GleamRule {
        check: check_no_unwrap,
        hint: Some("use pattern matching or let assert instead"),
        name: "no-unwrap",
    },
];

struct ParsedGleam {
    path: String,
    source: String,
    tree: Tree,
}

struct ParsedGleamToml {
    path: String,
    toml: toml::Value,
}

fn create_parser() -> Option<Parser> {
    let mut parser = Parser::new();
    match parser.set_language(&tree_sitter_gleam::LANGUAGE.into()) {
        Ok(()) => Some(parser),
        Err(_err) => None,
    }
}

fn get_constructor_name<'tree>(node: Node<'tree>, source: &'tree [u8]) -> Option<&'tree str> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "constructor_name" {
            match child.utf8_text(source) {
                Ok(text) => return Some(text),
                Err(_err) => return None,
            }
        }
    }
    None
}

fn pattern_matches_error(pattern_node: Node<'_>, source: &[u8]) -> bool {
    for child in pattern_node.children(&mut pattern_node.walk()) {
        if child.kind() == "case_clause_pattern" {
            for inner in child.children(&mut child.walk()) {
                if inner.kind() == "record_pattern"
                    && let Some(name) = get_constructor_name(inner, source)
                    && name == "Error"
                {
                    return true;
                }
            }
        }
    }
    false
}

fn get_last_expression(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "block" {
        let mut last_expr = None;
        for child in node.children(&mut node.walk()) {
            if child.kind() != "{" && child.kind() != "}" {
                last_expr = Some(child);
            }
        }
        return last_expr;
    }
    Some(node)
}

fn expression_returns_error(expr_node: Node<'_>, source: &[u8]) -> bool {
    let actual_expr = get_last_expression(expr_node);
    let Some(expr) = actual_expr else {
        return false;
    };

    if expr.kind() == "record"
        && let Some(name) = get_constructor_name(expr, source)
    {
        return name == "Error";
    }

    false
}

fn find_case_clauses<'tree>(node: Node<'tree>, clauses: &mut Vec<Node<'tree>>) {
    if node.kind() == "case_clause" {
        clauses.push(node);
    }
    for child in node.children(&mut node.walk()) {
        find_case_clauses(child, clauses);
    }
}

fn check_no_ignored_error(source: &str, tree: &Tree) -> Vec<String> {
    let mut errors = Vec::new();
    let source_bytes = source.as_bytes();

    let mut clauses = Vec::new();
    find_case_clauses(tree.root_node(), &mut clauses);

    for clause in clauses {
        let mut patterns_node = None;
        let mut body_node = None;
        let mut found_arrow = false;

        for child in clause.children(&mut clause.walk()) {
            match child.kind() {
                "case_clause_patterns" => patterns_node = Some(child),
                "->" => found_arrow = true,
                _ if found_arrow && body_node.is_none() => body_node = Some(child),
                _ => {}
            }
        }

        let Some(patterns) = patterns_node else {
            continue;
        };
        let Some(body) = body_node else {
            continue;
        };

        if pattern_matches_error(patterns, source_bytes)
            && !expression_returns_error(body, source_bytes)
        {
            let line = clause.start_position().row + 1;
            errors.push(format!("line {line}: Error matched but not returned"));
        }
    }

    errors
}

fn find_function_calls<'tree>(node: Node<'tree>, calls: &mut Vec<Node<'tree>>) {
    if node.kind() == "function_call" {
        calls.push(node);
    }
    for child in node.children(&mut node.walk()) {
        find_function_calls(child, calls);
    }
}

fn is_unwrap_call(node: Node<'_>, source: &[u8]) -> bool {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "field_access" {
            for inner in child.children(&mut child.walk()) {
                if inner.kind() == "label"
                    && let Ok(text) = inner.utf8_text(source)
                {
                    return text == "unwrap";
                }
            }
        }
    }
    false
}

const UNWRAP_MSG: &str = ".unwrap\x28)";

fn check_no_unwrap(source: &str, tree: &Tree) -> Vec<String> {
    let mut errors = Vec::new();
    let source_bytes = source.as_bytes();

    let mut calls = Vec::new();
    find_function_calls(tree.root_node(), &mut calls);

    for call in calls {
        if is_unwrap_call(call, source_bytes) {
            let line = call.start_position().row + 1;
            errors.push(format!("line {line}: {UNWRAP_MSG}"));
        }
    }

    errors
}

fn is_unpinned_version(version: &str) -> bool {
    let trimmed = version.trim();
    trimmed.starts_with(">=")
        || trimmed.starts_with('>')
        || trimmed.starts_with("<=")
        || trimmed.starts_with('<')
        || trimmed.starts_with("~>")
        || trimmed.contains(" and ")
        || trimmed.contains(" or ")
}

fn check_exact_versions(toml: &toml::Value, _path: &str) -> Vec<String> {
    let mut errors = Vec::new();

    let dep_sections = ["dependencies", "dev-dependencies"];

    for section in dep_sections {
        let Some(deps) = toml.get(section).and_then(toml::Value::as_table) else {
            continue;
        };

        for (name, value) in deps {
            let version = match value {
                toml::Value::String(ver) => Some(ver.as_str()),
                toml::Value::Table(table) => table.get("version").and_then(toml::Value::as_str),
                _ => None,
            };

            if let Some(ver) = version
                && is_unpinned_version(ver)
            {
                errors.push(format!("{section}.{name}: \"{ver}\" not pinned"));
            }
        }
    }

    errors
}

fn find(patterns: &[&str], excludes: &[&str]) -> (Vec<String>, Vec<AppError>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for pattern in patterns {
        match find_files(pattern, excludes) {
            Ok(found) => files.extend(found),
            Err(err) => errors.push(AppError::Glob {
                pattern: (*pattern).to_string(),
                message: err.to_string(),
            }),
        }
    }
    (files, errors)
}

fn parse_gleam_tomls(paths: Vec<String>) -> (Vec<ParsedGleamToml>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<toml::Value>(&content) {
                Ok(toml) => parsed.push(ParsedGleamToml { path, toml }),
                Err(err) => errors.push(AppError::Parse {
                    file: path,
                    message: err.to_string(),
                }),
            },
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn run_gleam_toml_rules(files: &[ParsedGleamToml], rules: &[GleamTomlRule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.toml, &file.path);
                if errors.is_empty() {
                    return None;
                }
                Some(Violation {
                    errors,
                    file: file.path.clone(),
                    hint: rule.hint.map(String::from),
                    rule: rule.name.to_string(),
                })
            })
        })
        .collect()
}

fn parse_gleam(paths: Vec<String>, parser: &mut Parser) -> (Vec<ParsedGleam>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => match parser.parse(&source, None) {
                Some(tree) => parsed.push(ParsedGleam { path, source, tree }),
                None => errors.push(AppError::Parse {
                    file: path,
                    message: "failed to parse".to_string(),
                }),
            },
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn run_gleam_rules(files: &[ParsedGleam], rules: &[GleamRule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.source, &file.tree);
                if errors.is_empty() {
                    return None;
                }
                Some(Violation {
                    errors,
                    file: file.path.clone(),
                    hint: rule.hint.map(String::from),
                    rule: rule.name.to_string(),
                })
            })
        })
        .collect()
}

pub fn check() -> CheckResult {
    let (gleam_paths, mut errors) = find(GLEAM_PATTERNS, EXCLUDES);
    let (gleam_toml_paths, toml_glob_errors) = find(GLEAM_TOML_PATTERNS, EXCLUDES);
    errors.extend(toml_glob_errors);

    let (parsed_gleam, gleam_errors) = if let Some(mut parser) = create_parser() {
        parse_gleam(gleam_paths, &mut parser)
    } else {
        errors.push(AppError::Parse {
            file: "gleam".to_string(),
            message: "failed to load gleam grammar".to_string(),
        });
        (Vec::new(), Vec::new())
    };
    let (parsed_tomls, toml_errors) = parse_gleam_tomls(gleam_toml_paths);
    errors.extend(gleam_errors);
    errors.extend(toml_errors);

    let mut violations = run_gleam_rules(&parsed_gleam, GLEAM_RULES);
    violations.extend(run_gleam_toml_rules(&parsed_tomls, GLEAM_TOML_RULES));

    CheckResult {
        errors,
        file_count: parsed_gleam.len() + parsed_tomls.len(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        let path = format!("fixtures/gleam/{name}");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    fn parse_fixture(name: &str) -> (String, Tree) {
        let source = read_fixture(name);
        let mut parser = create_parser().expect("parser should load");
        let tree = parser.parse(&source, None).expect("should parse");
        (source, tree)
    }

    #[test]
    fn exact_versions_detects_violation() {
        let content = read_fixture("exact-versions.toml");
        let toml: toml::Value = toml::from_str(&content).expect("valid toml");
        let errors = check_exact_versions(&toml, "test");
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn exact_versions_allows_valid() {
        let content = read_fixture("valid-versions.toml");
        let toml: toml::Value = toml::from_str(&content).expect("valid toml");
        let errors = check_exact_versions(&toml, "test");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_ignored_error_detects_violation() {
        let (source, tree) = parse_fixture("no-ignored-error.gleam");
        let errors = check_no_ignored_error(&source, &tree);
        assert_eq!(errors.len(), 4);
    }

    #[test]
    fn no_ignored_error_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-ignored-error.gleam");
        let errors = check_no_ignored_error(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_detects_violation() {
        let (source, tree) = parse_fixture("no-unwrap.gleam");
        let errors = check_no_unwrap(&source, &tree);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-unwrap.gleam");
        let errors = check_no_unwrap(&source, &tree);
        assert!(errors.is_empty());
    }
}
