use std::fs;

use oxc::allocator::Allocator;
use oxc::ast::ast::{Argument, ChainElement, Expression};
use oxc::parser::Parser;
use oxc::span::SourceType;
use oxc_ast_visit::{Visit, walk};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const PATTERNS: &[&str] = &["playwright/*.spec.ts"];
const EXCLUDES: &[&str] = &[];

struct PlaywrightVisitor<'src> {
    nested_call_arg_errors: Vec<String>,
    page_errors: Vec<String>,
    page_request_errors: Vec<String>,
    request_errors: Vec<String>,
    path: &'src str,
    source: &'src str,
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

fn line_number(source: &str, offset: u32) -> usize {
    let Ok(offset_usize) = usize::try_from(offset) else {
        return 1;
    };
    source.get(..offset_usize).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

fn create_visitor<'src>(source: &'src str, path: &'src str) -> PlaywrightVisitor<'src> {
    PlaywrightVisitor {
        nested_call_arg_errors: Vec::new(),
        page_errors: Vec::new(),
        page_request_errors: Vec::new(),
        request_errors: Vec::new(),
        path,
        source,
    }
}

fn is_nested_call_expression(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::CallExpression(_) => true,
        Expression::AwaitExpression(inner) => is_nested_call_expression(&inner.argument),
        Expression::ChainExpression(inner) => match &inner.expression {
            ChainElement::CallExpression(_) => true,
            ChainElement::TSNonNullExpression(non_null) => {
                is_nested_call_expression(&non_null.expression)
            }
            _ => false,
        },
        Expression::ParenthesizedExpression(inner) => is_nested_call_expression(&inner.expression),
        Expression::TSAsExpression(inner) => is_nested_call_expression(&inner.expression),
        Expression::TSNonNullExpression(inner) => is_nested_call_expression(&inner.expression),
        Expression::TSTypeAssertion(inner) => is_nested_call_expression(&inner.expression),
        _ => false,
    }
}

fn is_nested_call_argument(arg: &Argument<'_>) -> bool {
    match arg {
        Argument::SpreadElement(spread) => is_nested_call_expression(&spread.argument),
        Argument::CallExpression(_) => true,
        Argument::AwaitExpression(inner) => is_nested_call_expression(&inner.argument),
        Argument::ChainExpression(inner) => match &inner.expression {
            ChainElement::CallExpression(_) => true,
            ChainElement::TSNonNullExpression(non_null) => {
                is_nested_call_expression(&non_null.expression)
            }
            _ => false,
        },
        Argument::ParenthesizedExpression(inner) => is_nested_call_expression(&inner.expression),
        Argument::TSAsExpression(inner) => is_nested_call_expression(&inner.expression),
        Argument::TSNonNullExpression(inner) => is_nested_call_expression(&inner.expression),
        Argument::TSTypeAssertion(inner) => is_nested_call_expression(&inner.expression),
        _ => false,
    }
}

impl<'src> Visit<'src> for PlaywrightVisitor<'src> {
    fn visit_call_expression(&mut self, expr: &oxc::ast::ast::CallExpression<'src>) {
        if expr.arguments.iter().any(is_nested_call_argument) {
            let line = line_number(self.source, expr.span.start);
            self.nested_call_arg_errors
                .push(format!("line {line}: nested call argument"));
        }
        walk::walk_call_expression(self, expr);
    }

    fn visit_expression(&mut self, expr: &Expression<'src>) {
        if let Expression::StaticMemberExpression(member) = expr
            && let Expression::Identifier(id) = &member.object
        {
            let line = line_number(self.source, member.span.start);
            let prop = &member.property.name;

            match id.name.as_str() {
                "request" => {
                    self.request_errors
                        .push(format!("line {line}: request.{prop}"));
                }
                "page" => {
                    if prop.as_str() == "request" {
                        self.page_request_errors
                            .push(format!("line {line}: page.request"));
                    }
                    self.page_errors.push(format!("line {line}: page.{prop}"));
                }
                _ => {}
            }
        }
        walk::walk_expression(self, expr);
    }
}

fn collect_violations(visitor: &PlaywrightVisitor<'_>) -> Vec<Violation> {
    let is_client = visitor.path.contains("playwright/client/");
    let is_server = visitor.path.contains("playwright/server/");

    let mut rules: Vec<(&str, &Vec<String>, Option<&str>)> = vec![(
        "no-nested-call-args",
        &visitor.nested_call_arg_errors,
        Some("assign the inner call to a const before passing it"),
    )];

    if is_client {
        rules.push((
            "no-request",
            &visitor.request_errors,
            Some("Use page for client tests, use request in server tests"),
        ));
        rules.push((
            "no-page-request",
            &visitor.page_request_errors,
            Some("Use page for client tests, use request in server tests"),
        ));
    }

    if is_server {
        rules.push((
            "no-page",
            &visitor.page_errors,
            Some("Use request for server tests, use page in client tests"),
        ));
    }

    rules
        .into_iter()
        .filter_map(|(name, errors, hint)| {
            if errors.is_empty() {
                return None;
            }
            Some(Violation {
                errors: errors.clone(),
                file: visitor.path.to_string(),
                hint: hint.map(str::to_string),
                rule: name.to_string(),
            })
        })
        .collect()
}

fn check_files(paths: Vec<String>) -> (Vec<Violation>, Vec<AppError>, usize) {
    let mut all_violations = Vec::new();
    let mut errors = Vec::new();
    let mut file_count = 0;

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => {
                file_count += 1;
                let allocator = Allocator::default();
                let source_type = SourceType::ts();
                let parser = Parser::new(&allocator, &source, source_type);
                let result = parser.parse();

                if result.panicked {
                    errors.push(AppError::Parse {
                        file: path,
                        message: "Parser panicked".to_string(),
                    });
                    continue;
                }

                let mut visitor = create_visitor(&source, &path);
                visitor.visit_program(&result.program);
                let violations = collect_violations(&visitor);
                all_violations.extend(violations);
            }
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (all_violations, errors, file_count)
}

pub fn check() -> CheckResult {
    let (paths, mut errors) = find(PATTERNS, EXCLUDES);
    let (violations, check_errors, file_count) = check_files(paths);
    errors.extend(check_errors);

    CheckResult {
        skipped: Vec::new(),
        errors,
        file_count,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        let path = format!("tests/fixtures/playwright/{name}");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    fn parse_fixture_as(name: &str, virtual_path: &str) -> Vec<Violation> {
        let source = read_fixture(&format!("{name}.spec.ts"));
        let allocator = Allocator::default();
        let source_type = SourceType::ts();
        let parser = Parser::new(&allocator, &source, source_type);
        let result = parser.parse();
        let mut visitor = create_visitor(&source, virtual_path);
        visitor.visit_program(&result.program);
        collect_violations(&visitor)
    }

    fn has_violation(violations: &[Violation], rule: &str) -> bool {
        violations.iter().any(|violation| violation.rule == rule)
    }

    #[test]
    fn no_request_detects_violation() {
        let violations = parse_fixture_as("no-request", "playwright/client/test.spec.ts");
        assert!(has_violation(&violations, "no-request"));
    }

    #[test]
    fn no_request_allows_valid() {
        let violations = parse_fixture_as("valid-no-request", "playwright/client/test.spec.ts");
        assert!(!has_violation(&violations, "no-request"));
    }

    #[test]
    fn no_page_request_detects_violation() {
        let violations = parse_fixture_as("no-page-request", "playwright/client/test.spec.ts");
        assert!(has_violation(&violations, "no-page-request"));
    }

    #[test]
    fn no_page_request_allows_valid() {
        let violations =
            parse_fixture_as("valid-no-page-request", "playwright/client/test.spec.ts");
        assert!(!has_violation(&violations, "no-page-request"));
    }

    #[test]
    fn no_page_detects_violation() {
        let violations = parse_fixture_as("no-page", "playwright/server/test.spec.ts");
        assert!(has_violation(&violations, "no-page"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_page_allows_valid() {
        let violations = parse_fixture_as("valid-no-page", "playwright/server/test.spec.ts");
        assert!(!has_violation(&violations, "no-page"));
    }

    #[test]
    fn no_nested_call_args_detects_violation() {
        let violations = parse_fixture_as(
            "no-nested-call-args",
            "playwright/server/no-nested-call-args.spec.ts",
        );
        assert!(has_violation(&violations, "no-nested-call-args"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_nested_call_args_allows_valid() {
        let violations = parse_fixture_as(
            "valid-no-nested-call-args",
            "playwright/server/valid-no-nested-call-args.spec.ts",
        );
        assert!(!has_violation(&violations, "no-nested-call-args"));
    }
}
