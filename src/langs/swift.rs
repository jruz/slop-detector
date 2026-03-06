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

const RULES: &[Rule] = &[Rule {
    check: check_no_try_question_mark,
    hint: Some("use do/catch to handle errors instead of silencing them"),
    name: "no-try-question-mark",
}];

struct ParsedSwift {
    path: String,
    source: String,
    tree: Tree,
}

fn create_parser() -> Result<Parser, tree_sitter::LanguageError> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_swift::LANGUAGE.into())?;
    Ok(parser)
}

fn find_nodes_by_kind<'tree>(node: Node<'tree>, kind: &str, results: &mut Vec<Node<'tree>>) {
    if node.kind() == kind {
        results.push(node);
    }
    for child in node.children(&mut node.walk()) {
        find_nodes_by_kind(child, kind, results);
    }
}

fn has_question_mark(node: Node<'_>) -> bool {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "try_operator" {
            for inner in child.children(&mut child.walk()) {
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
    Ok(text.lines().next().map_or("try?", |line| line))
}

fn check_no_try_question_mark(source: &str, tree: &Tree) -> Vec<String> {
    let mut try_expressions = Vec::new();
    find_nodes_by_kind(tree.root_node(), "try_expression", &mut try_expressions);

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
}
