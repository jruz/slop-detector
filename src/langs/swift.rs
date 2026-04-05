use std::fs;

use tree_sitter::{Node, Parser, Tree};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[".build", "fixtures"];
const PATTERNS: &[&str] = &["*.swift"];

struct Rule {
    check: fn(source: &str, tree: &Tree) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_no_guard_return,
        hint: Some("throw or log before returning instead of silently discarding"),
        name: "no-guard-return",
    },
    Rule {
        check: check_no_silent_catch,
        hint: Some("handle the error instead of silently discarding it"),
        name: "no-silent-catch",
    },
    Rule {
        check: check_no_silent_if_let,
        hint: Some("throw or log before returning instead of silently discarding"),
        name: "no-silent-if-let",
    },
    Rule {
        check: check_no_try_question_mark,
        hint: Some("use do/catch to handle errors instead of silencing them"),
        name: "no-try-question-mark",
    },
    Rule {
        check: check_no_nested_call_args,
        hint: Some("assign the inner call to a let before passing it"),
        name: "no-nested-call-args",
    },
];

struct ParsedSwift {
    path: String,
    source: String,
    tree: Tree,
}

fn create_parser() -> Result<Parser, tree_sitter::LanguageError> {
    let mut parser = Parser::new();
    let language = tree_sitter_swift::LANGUAGE.into();
    parser.set_language(&language)?;
    Ok(parser)
}

fn find_nodes_by_kind<'tree>(node: Node<'tree>, kind: &str, results: &mut Vec<Node<'tree>>) {
    if node.kind() == kind {
        results.push(node);
    }
    let mut walker = node.walk();
    for child in node.children(&mut walker) {
        find_nodes_by_kind(child, kind, results);
    }
}

fn has_question_mark(node: Node<'_>) -> bool {
    let mut node_walker = node.walk();
    for child in node.children(&mut node_walker) {
        if child.kind() == "try_operator" {
            let mut child_walker = child.walk();
            for inner in child.children(&mut child_walker) {
                if inner.kind() == "?" {
                    return true;
                }
            }
        }
    }
    false
}

fn node_first_line<'src>(
    node: Node<'_>,
    source: &'src str,
) -> Result<&'src str, std::str::Utf8Error> {
    let start = node.start_byte();
    let end = node.end_byte();
    let Some(slice) = source.as_bytes().get(start..end) else {
        return Ok("try?");
    };
    let text = std::str::from_utf8(slice)?;
    let first_line = text.lines().next().map_or("try?", |line| line);
    Ok(first_line)
}

fn is_silent_return(statements: Node<'_>) -> bool {
    let mut walker = statements.walk();
    let mut children = statements.children(&mut walker);
    let Some(first) = children.next() else {
        return false;
    };
    if children.next().is_some() || first.kind() != "control_transfer_statement" {
        return false;
    }

    let mut first_walker = first.walk();
    let kinds: Vec<&str> = first
        .children(&mut first_walker)
        .map(|child| child.kind())
        .collect();

    matches!(kinds.as_slice(), ["return"] | ["return", "nil"])
}

fn find_else_body(node: Node<'_>) -> Option<Node<'_>> {
    let mut found_else = false;
    let mut walker = node.walk();
    for child in node.children(&mut walker) {
        if child.kind() == "else" {
            found_else = true;
            continue;
        }
        if found_else && child.kind() == "statements" {
            return Some(child);
        }
    }
    None
}

fn check_no_guard_return(source: &str, tree: &Tree) -> Vec<String> {
    let mut guards = Vec::new();
    let root = tree.root_node();
    find_nodes_by_kind(root, "guard_statement", &mut guards);

    let mut errors = Vec::new();
    for node in &guards {
        let Some(body) = find_else_body(*node) else {
            continue;
        };
        if !is_silent_return(body) {
            continue;
        }
        let line = node.start_position().row + 1;
        match node_first_line(*node, source) {
            Ok(text) => errors.push(format!("line {line}: {text}")),
            Err(utf8_err) => errors.push(format!("UTF-8 error: {utf8_err}")),
        }
    }
    errors
}

fn check_no_silent_catch(source: &str, tree: &Tree) -> Vec<String> {
    let mut catches = Vec::new();
    let root = tree.root_node();
    find_nodes_by_kind(root, "catch_block", &mut catches);

    let mut errors = Vec::new();
    for node in &catches {
        let mut walker = node.walk();
        let statements = node
            .children(&mut walker)
            .find(|child| child.kind() == "statements");
        let is_silent = match statements {
            None => true,
            Some(stmts) => is_silent_return(stmts),
        };
        if !is_silent {
            continue;
        }
        let line = node.start_position().row + 1;
        match node_first_line(*node, source) {
            Ok(text) => errors.push(format!("line {line}: {text}")),
            Err(utf8_err) => errors.push(format!("UTF-8 error: {utf8_err}")),
        }
    }
    errors
}

fn check_no_silent_if_let(source: &str, tree: &Tree) -> Vec<String> {
    let mut if_stmts = Vec::new();
    let root = tree.root_node();
    find_nodes_by_kind(root, "if_statement", &mut if_stmts);

    let mut errors = Vec::new();
    for node in &if_stmts {
        let mut walker = node.walk();
        let is_if_let = node
            .children(&mut walker)
            .any(|child| child.kind() == "value_binding_pattern");
        if !is_if_let {
            continue;
        }
        let Some(body) = find_else_body(*node) else {
            continue;
        };
        if !is_silent_return(body) {
            continue;
        }
        let line = node.start_position().row + 1;
        match node_first_line(*node, source) {
            Ok(text) => errors.push(format!("line {line}: {text}")),
            Err(utf8_err) => errors.push(format!("UTF-8 error: {utf8_err}")),
        }
    }
    errors
}

fn check_no_try_question_mark(source: &str, tree: &Tree) -> Vec<String> {
    let mut try_expressions = Vec::new();
    let root = tree.root_node();
    find_nodes_by_kind(root, "try_expression", &mut try_expressions);

    let mut errors = Vec::new();
    for node in &try_expressions {
        if !has_question_mark(*node) {
            continue;
        }
        let line = node.start_position().row + 1;
        match node_first_line(*node, source) {
            Ok(text) => errors.push(format!("line {line}: {text}")),
            Err(utf8_err) => errors.push(format!("UTF-8 error: {utf8_err}")),
        }
    }
    errors
}

fn check_no_nested_call_args(_source: &str, tree: &Tree) -> Vec<String> {
    let mut calls = Vec::new();
    let root = tree.root_node();
    find_nodes_by_kind(root, "call_expression", &mut calls);

    let mut errors = Vec::new();
    for node in &calls {
        let mut node_walker = node.walk();
        let call_suffix = node
            .children(&mut node_walker)
            .find(|child| child.kind() == "call_suffix");
        let has_nested = call_suffix.is_some_and(|suffix| {
            let mut suffix_walker = suffix.walk();
            let value_arguments = suffix
                .children(&mut suffix_walker)
                .find(|child| child.kind() == "value_arguments");
            value_arguments.is_some_and(|arguments| {
                let mut arguments_walker = arguments.walk();
                arguments
                    .children(&mut arguments_walker)
                    .filter(|child| child.kind() == "value_argument")
                    .any(|argument| {
                        argument
                            .child_by_field_name("value")
                            .is_some_and(|value| value.kind() == "call_expression")
                    })
            })
        });

        if has_nested {
            let line = node.start_position().row + 1;
            errors.push(format!("line {line}: nested call argument"));
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

fn parse_swift(paths: Vec<String>, parser: &mut Parser) -> (Vec<ParsedSwift>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => match parser.parse(&source, None) {
                Some(tree) => parsed.push(ParsedSwift { path, source, tree }),
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

fn run_rules(files: &[ParsedSwift], rules: &[Rule]) -> Vec<Violation> {
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
    let (paths, mut errors) = find(PATTERNS, EXCLUDES);

    let (parsed, parse_errors) = match create_parser() {
        Ok(mut parser) => parse_swift(paths, &mut parser),
        Err(err) => {
            errors.push(AppError::Parse {
                file: "swift".to_string(),
                message: err.to_string(),
            });
            (Vec::new(), Vec::new())
        }
    };
    errors.extend(parse_errors);

    let violations = run_rules(&parsed, RULES);

    CheckResult {
        errors,
        file_count: parsed.len(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        let path = format!("fixtures/swift/{name}");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    fn parse_fixture(name: &str) -> (String, Tree) {
        let source = read_fixture(name);
        let mut parser = create_parser().expect("parser should load");
        let tree = parser.parse(&source, None).expect("should parse");
        (source, tree)
    }

    #[test]
    fn no_guard_return_detects_violation() {
        let (source, tree) = parse_fixture("no-guard-return.swift");
        let errors = check_no_guard_return(&source, &tree);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_guard_return_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-guard-return.swift");
        let errors = check_no_guard_return(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_silent_catch_detects_violation() {
        let (source, tree) = parse_fixture("no-silent-catch.swift");
        let errors = check_no_silent_catch(&source, &tree);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_silent_catch_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-silent-catch.swift");
        let errors = check_no_silent_catch(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_silent_if_let_detects_violation() {
        let (source, tree) = parse_fixture("no-silent-if-let.swift");
        let errors = check_no_silent_if_let(&source, &tree);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_silent_if_let_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-silent-if-let.swift");
        let errors = check_no_silent_if_let(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_try_question_mark_detects_violation() {
        let (source, tree) = parse_fixture("no-try-question-mark.swift");
        let errors = check_no_try_question_mark(&source, &tree);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_try_question_mark_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-try-question-mark.swift");
        let errors = check_no_try_question_mark(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_nested_call_args_detects_violation() {
        let (source, tree) = parse_fixture("no-nested-call-args.swift");
        let errors = check_no_nested_call_args(&source, &tree);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_nested_call_args_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-nested-call-args.swift");
        let errors = check_no_nested_call_args(&source, &tree);
        assert!(errors.is_empty());
    }
}
