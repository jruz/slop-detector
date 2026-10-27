use std::fs;

use hcl::expr::{Traversal, TraversalOperator};
use hcl::{Body, Expression};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[];
const PATTERNS: &[&str] = &["nomad/*.hcl", "*.hcl"];

struct ParsedNomadFile {
    body: Body,
    path: String,
}

struct Rule {
    name: &'static str,
    hint: Option<&'static str>,
    check: fn(body: &Body, path: &str) -> Vec<String>,
}

const RULES: &[Rule] = &[
    Rule {
        name: "no-hardcoded-env",
        hint: Some("use var.* references instead of literal values"),
        check: check_no_hardcoded_env,
    },
    Rule {
        name: "env-var-naming",
        hint: Some("var references must be UPPER_SNAKE_CASE"),
        check: check_env_var_naming,
    },
    Rule {
        name: "env-var-mismatch",
        hint: Some("env key and var name should match"),
        check: check_env_var_mismatch,
    },
];

fn is_literal(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::String(_) | Expression::Number(_) | Expression::Bool(_)
    )
}

fn collect_env_violations(body: &Body) -> Vec<String> {
    let env_violations: Vec<String> = body
        .blocks()
        .filter(|block| block.identifier.as_str() == "env")
        .flat_map(|block| {
            block
                .body
                .attributes()
                .filter(|attr| is_literal(&attr.expr))
                .map(|attr| {
                    let key = attr.key.as_str();
                    format!("{key} = {}", format_expr(&attr.expr))
                })
        })
        .collect();

    let nested_violations: Vec<String> = body
        .blocks()
        .filter(|block| block.identifier.as_str() != "env")
        .flat_map(|block| collect_env_violations(&block.body))
        .collect();

    [env_violations, nested_violations].concat()
}

fn format_expr(expr: &Expression) -> String {
    match expr {
        Expression::String(val) => format!("\"{val}\""),
        Expression::Number(val) => val.to_string(),
        Expression::Bool(val) => val.to_string(),
        _ => String::new(),
    }
}

fn check_no_hardcoded_env(body: &Body, _path: &str) -> Vec<String> {
    collect_env_violations(body)
}

fn extract_var_traversal(expression: &Expression) -> Option<(&str, &str)> {
    let Expression::Traversal(traversal) = expression else {
        return None;
    };
    let Traversal {
        expr: inner,
        operators,
    } = traversal.as_ref();
    let Expression::Variable(var) = inner else {
        return None;
    };
    if var.as_str() != "var" {
        return None;
    }
    let [TraversalOperator::GetAttr(ident)] = operators.as_slice() else {
        return None;
    };
    Some((var.as_str(), ident.as_str()))
}

fn is_upper_snake_case(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn collect_var_naming_violations(body: &Body) -> Vec<String> {
    let local: Vec<String> = body
        .blocks()
        .filter(|block| block.identifier.as_str() == "env")
        .flat_map(|block| {
            block.body.attributes().filter_map(|attr| {
                let (_, var_name) = extract_var_traversal(&attr.expr)?;
                if is_upper_snake_case(var_name) {
                    return None;
                }
                Some(format!(
                    "{} = var.{var_name} (not UPPER_SNAKE_CASE)",
                    attr.key.as_str()
                ))
            })
        })
        .collect();

    let nested: Vec<String> = body
        .blocks()
        .filter(|block| block.identifier.as_str() != "env")
        .flat_map(|block| collect_var_naming_violations(&block.body))
        .collect();

    [local, nested].concat()
}

fn collect_var_mismatch_violations(body: &Body) -> Vec<String> {
    let local: Vec<String> = body
        .blocks()
        .filter(|block| block.identifier.as_str() == "env")
        .flat_map(|block| {
            block.body.attributes().filter_map(|attr| {
                let (_, var_name) = extract_var_traversal(&attr.expr)?;
                let key = attr.key.as_str();
                if !is_upper_snake_case(var_name) {
                    return None;
                }
                if key == var_name {
                    return None;
                }
                Some(format!("{key} = var.{var_name} (names differ)"))
            })
        })
        .collect();

    let nested: Vec<String> = body
        .blocks()
        .filter(|block| block.identifier.as_str() != "env")
        .flat_map(|block| collect_var_mismatch_violations(&block.body))
        .collect();

    [local, nested].concat()
}

fn check_env_var_naming(body: &Body, _path: &str) -> Vec<String> {
    collect_var_naming_violations(body)
}

fn check_env_var_mismatch(body: &Body, _path: &str) -> Vec<String> {
    collect_var_mismatch_violations(body)
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

fn parse(paths: Vec<String>) -> (Vec<ParsedNomadFile>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match hcl::from_str(&content) {
                Ok(body) => parsed.push(ParsedNomadFile { body, path }),
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

fn run_rules(files: &[ParsedNomadFile], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().flat_map(|rule| {
                let errors = (rule.check)(&file.body, &file.path);
                if errors.is_empty() {
                    vec![]
                } else {
                    vec![Violation {
                        errors,
                        file: file.path.clone(),
                        hint: rule.hint.map(String::from),
                        rule: rule.name.to_string(),
                    }]
                }
            })
        })
        .collect()
}

pub fn check() -> CheckResult {
    let (paths, mut errors) = find(PATTERNS, EXCLUDES);
    let (parsed, parse_errors) = parse(paths);
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

    fn parse_fixture(name: &str) -> Body {
        let path = format!("fixtures/nomad/{name}.hcl");
        let content = fs::read_to_string(&path).expect("fixture should exist");
        hcl::from_str(&content).expect("valid hcl")
    }

    #[test]
    fn no_hardcoded_env_detects_violation() {
        let body = parse_fixture("no-hardcoded-env");
        let errors = check_no_hardcoded_env(&body, "test");
        assert_eq!(errors.len(), 4);
    }

    #[test]
    fn no_hardcoded_env_allows_valid() {
        let body = parse_fixture("valid-no-hardcoded-env");
        let errors = check_no_hardcoded_env(&body, "test");
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn env_var_naming_detects_violation() {
        let body = parse_fixture("env-var-naming");
        let errors = check_env_var_naming(&body, "test");
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn env_var_naming_allows_valid() {
        let body = parse_fixture("valid-env-var-naming");
        let errors = check_env_var_naming(&body, "test");
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn env_var_mismatch_detects_violation() {
        let body = parse_fixture("env-var-mismatch");
        let errors = check_env_var_mismatch(&body, "test");
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn env_var_mismatch_allows_valid() {
        let body = parse_fixture("valid-env-var-mismatch");
        let errors = check_env_var_mismatch(&body, "test");
        assert_eq!(errors, Vec::<String>::new());
    }
}
