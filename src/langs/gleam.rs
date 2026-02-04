use std::fs;

use tree_sitter::{Node, Parser, Tree};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["build", "fixtures"];
const GLEAM_PATTERNS: &[&str] = &["*.gleam"];
const GLEAM_TOML_PATTERNS: &[&str] = &["gleam.toml"];

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
        check: check_no_discarded_error_fields,
        hint: Some("log or propagate error details instead of discarding them"),
        name: "no-discarded-error-fields",
    },
    GleamRule {
        check: check_no_ignored_error,
        hint: Some("handle or propagate the error instead of ignoring it"),
        name: "no-ignored-error",
    },
    GleamRule {
        check: check_no_nested_option,
        hint: Some("use `use value <- option.then(expr)` to flatten"),
        name: "no-nested-option",
    },
    GleamRule {
        check: check_no_nested_result,
        hint: Some("use `use value <- result.try(expr)` to flatten"),
        name: "no-nested-result",
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

fn create_parser() -> Result<Parser, tree_sitter::LanguageError> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_gleam::LANGUAGE.into())?;
    Ok(parser)
}

fn get_constructor_name<'tree>(
    node: Node<'tree>,
    source: &'tree [u8],
) -> Result<Option<&'tree str>, std::str::Utf8Error> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "constructor_name" {
            return Ok(Some(child.utf8_text(source)?));
        }
    }
    Ok(None)
}

fn extract_error_binding<'tree>(
    pattern_node: Node<'tree>,
    source: &'tree [u8],
) -> Result<Option<&'tree str>, std::str::Utf8Error> {
    for child in pattern_node.children(&mut pattern_node.walk()) {
        if child.kind() == "case_clause_pattern" {
            for inner in child.children(&mut child.walk()) {
                if inner.kind() == "record_pattern"
                    && get_constructor_name(inner, source)? == Some("Error")
                {
                    for arg_container in inner.children(&mut inner.walk()) {
                        if arg_container.kind() == "record_pattern_arguments" {
                            for arg in arg_container.children(&mut arg_container.walk()) {
                                if arg.kind() == "record_pattern_argument" {
                                    for binding in arg.children(&mut arg.walk()) {
                                        if binding.kind() == "discard"
                                            || binding.kind() == "identifier"
                                        {
                                            return Ok(Some(binding.utf8_text(source)?));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn find_case_clauses<'tree>(node: Node<'tree>, clauses: &mut Vec<Node<'tree>>) {
    if node.kind() == "case_clause" {
        clauses.push(node);
    }
    for child in node.children(&mut node.walk()) {
        find_case_clauses(child, clauses);
    }
}

fn has_all_discarded_fields(node: Node<'_>) -> bool {
    let mut has_arguments = false;
    let mut all_discards = true;

    for child in node.children(&mut node.walk()) {
        if child.kind() == "record_pattern_arguments" {
            for arg in child.children(&mut child.walk()) {
                if arg.kind() == "record_pattern_argument" {
                    has_arguments = true;
                    let mut is_discard = false;
                    for inner in arg.children(&mut arg.walk()) {
                        if inner.kind() == "discard" {
                            is_discard = true;
                            break;
                        }
                    }
                    if !is_discard {
                        all_discards = false;
                    }
                }
            }
        }
    }

    has_arguments && all_discards
}

fn find_discarded_error_fields(
    clause: Node<'_>,
    source: &[u8],
) -> Result<Vec<(usize, String)>, std::str::Utf8Error> {
    let mut results = Vec::new();

    for child in clause.children(&mut clause.walk()) {
        if child.kind() == "case_clause_patterns" {
            for pattern in child.children(&mut child.walk()) {
                if pattern.kind() == "case_clause_pattern" {
                    for inner in pattern.children(&mut pattern.walk()) {
                        if inner.kind() == "record_pattern"
                            && get_constructor_name(inner, source)? == Some("Error")
                        {
                            for arg_container in inner.children(&mut inner.walk()) {
                                if arg_container.kind() == "record_pattern_arguments" {
                                    for arg in arg_container.children(&mut arg_container.walk()) {
                                        if arg.kind() == "record_pattern_argument" {
                                            for binding in arg.children(&mut arg.walk()) {
                                                if binding.kind() == "record_pattern"
                                                    && has_all_discarded_fields(binding)
                                                    && let Some(name) =
                                                        get_constructor_name(binding, source)?
                                                {
                                                    let line = binding.start_position().row + 1;
                                                    results.push((line, name.to_string()));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

fn check_no_discarded_error_fields(source: &str, tree: &Tree) -> Vec<String> {
    let source_bytes = source.as_bytes();
    let mut clauses = Vec::new();
    find_case_clauses(tree.root_node(), &mut clauses);

    let mut errors = Vec::new();
    for clause in clauses {
        match find_discarded_error_fields(clause, source_bytes) {
            Ok(findings) => {
                for (line, name) in findings {
                    errors.push(format!("line {line}: Error({name}) discards all fields"));
                }
            }
            Err(utf8_err) => {
                errors.push(format!("UTF-8 error: {utf8_err}"));
            }
        }
    }
    errors
}
fn check_clause_for_ignored_error(
    clause: Node<'_>,
    source_bytes: &[u8],
) -> Result<Option<String>, std::str::Utf8Error> {
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

    let (Some(patterns), Some(body)) = (patterns_node, body_node) else {
        return Ok(None);
    };

    let Some(binding) = extract_error_binding(patterns, source_bytes)? else {
        return Ok(None);
    };

    let body_str = body.utf8_text(source_bytes)?;

    if body_str.contains("panic as") {
        return Ok(None);
    }

    let is_discarded = binding.starts_with('_');
    let is_unused = !body_str.contains(binding);

    if is_discarded || is_unused {
        let line = clause.start_position().row + 1;
        return Ok(Some(format!("line {line}: Error({binding}) not used")));
    }

    Ok(None)
}

fn check_no_ignored_error(source: &str, tree: &Tree) -> Vec<String> {
    let source_bytes = source.as_bytes();
    let mut clauses = Vec::new();
    find_case_clauses(tree.root_node(), &mut clauses);

    let mut errors = Vec::new();
    for clause in clauses {
        match check_clause_for_ignored_error(clause, source_bytes) {
            Ok(Some(error)) => errors.push(error),
            Ok(None) => {}
            Err(utf8_err) => {
                errors.push(format!("UTF-8 error: {utf8_err}"));
            }
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

fn find_case_expressions<'tree>(node: Node<'tree>, cases: &mut Vec<Node<'tree>>) {
    if node.kind() == "case" {
        cases.push(node);
    }
    for child in node.children(&mut node.walk()) {
        find_case_expressions(child, cases);
    }
}

fn case_has_pattern(node: Node<'_>, source: &[u8], pattern_name: &str) -> bool {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "case_clauses" {
            for clause in child.children(&mut child.walk()) {
                if clause.kind() == "case_clause" {
                    for part in clause.children(&mut clause.walk()) {
                        if part.kind() == "case_clause_patterns" {
                            for pattern in part.children(&mut part.walk()) {
                                if pattern.kind() == "case_clause_pattern" {
                                    for inner in pattern.children(&mut pattern.walk()) {
                                        if inner.kind() == "record_pattern"
                                            && let Ok(Some(name)) =
                                                get_constructor_name(inner, source)
                                            && name == pattern_name
                                        {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn find_cases_in_body<'tree>(
    body: Node<'tree>,
    cases: &mut Vec<Node<'tree>>,
    source: &[u8],
    ok_pattern: &str,
    err_pattern: &str,
) {
    if body.kind() == "case"
        && case_has_pattern(body, source, ok_pattern)
        && case_has_pattern(body, source, err_pattern)
    {
        cases.push(body);
        return;
    }
    for child in body.children(&mut body.walk()) {
        if child.kind() == "case"
            && case_has_pattern(child, source, ok_pattern)
            && case_has_pattern(child, source, err_pattern)
        {
            cases.push(child);
        } else {
            find_cases_in_body(child, cases, source, ok_pattern, err_pattern);
        }
    }
}

fn find_nested_cases_in_case<'tree>(
    case_node: Node<'tree>,
    nested: &mut Vec<Node<'tree>>,
    source: &[u8],
    ok_pattern: &str,
    err_pattern: &str,
) {
    for child in case_node.children(&mut case_node.walk()) {
        if child.kind() == "case_clauses" {
            for clause in child.children(&mut child.walk()) {
                if clause.kind() == "case_clause" {
                    let mut found_arrow = false;
                    for part in clause.children(&mut clause.walk()) {
                        match (part.kind(), found_arrow) {
                            ("->", _) => found_arrow = true,
                            (_, true) => {
                                find_cases_in_body(part, nested, source, ok_pattern, err_pattern);
                            }
                            (_, false) => {}
                        }
                    }
                }
            }
        }
    }
}

fn check_nested_case(
    source: &str,
    tree: &Tree,
    ok_pattern: &str,
    err_pattern: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    let source_bytes = source.as_bytes();

    let mut cases = Vec::new();
    find_case_expressions(tree.root_node(), &mut cases);

    let top_level_cases: Vec<_> = cases
        .iter()
        .filter(|case| {
            case_has_pattern(**case, source_bytes, ok_pattern)
                && case_has_pattern(**case, source_bytes, err_pattern)
        })
        .collect();

    for case in top_level_cases {
        let mut level1_nested = Vec::new();
        find_nested_cases_in_case(
            *case,
            &mut level1_nested,
            source_bytes,
            ok_pattern,
            err_pattern,
        );

        for nested_case in level1_nested {
            let mut level2_nested = Vec::new();
            find_nested_cases_in_case(
                nested_case,
                &mut level2_nested,
                source_bytes,
                ok_pattern,
                err_pattern,
            );

            for deeply_nested in level2_nested {
                let line = deeply_nested.start_position().row + 1;
                errors.push(format!(
                    "line {line}: case nested 2 levels deep ({ok_pattern}/{err_pattern})"
                ));
            }
        }
    }

    errors
}

fn check_no_nested_result(source: &str, tree: &Tree) -> Vec<String> {
    check_nested_case(source, tree, "Ok", "Error")
}
fn check_no_nested_option(source: &str, tree: &Tree) -> Vec<String> {
    check_nested_case(source, tree, "Some", "None")
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
    match find_files(patterns, excludes) {
        Ok(found) => (found, vec![]),
        Err(err) => (
            vec![],
            vec![AppError::Glob {
                pattern: patterns.join(", "),
                message: err.to_string(),
            }],
        ),
    }
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

    let (parsed_gleam, gleam_errors) = match create_parser() {
        Ok(mut parser) => parse_gleam(gleam_paths, &mut parser),
        Err(err) => {
            errors.push(AppError::Parse {
                file: "gleam".to_string(),
                message: err.to_string(),
            });
            (Vec::new(), Vec::new())
        }
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
    fn no_discarded_error_fields_detects_violation() {
        let (source, tree) = parse_fixture("no-discarded-error-fields.gleam");
        let errors = check_no_discarded_error_fields(&source, &tree);
        assert_eq!(errors.len(), 5);
    }

    #[test]
    fn no_discarded_error_fields_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-discarded-error-fields.gleam");
        let errors = check_no_discarded_error_fields(&source, &tree);
        assert!(errors.is_empty());
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
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_ignored_error_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-ignored-error.gleam");
        let errors = check_no_ignored_error(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_nested_option_detects_violation() {
        let (source, tree) = parse_fixture("no-nested-option.gleam");
        let errors = check_no_nested_option(&source, &tree);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_nested_option_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-nested-option.gleam");
        let errors = check_no_nested_option(&source, &tree);
        assert!(errors.is_empty());
    }
    #[test]
    fn no_nested_result_detects_violation() {
        let (source, tree) = parse_fixture("no-nested-result.gleam");
        let errors = check_no_nested_result(&source, &tree);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_nested_result_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-nested-result.gleam");
        let errors = check_no_nested_result(&source, &tree);
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
