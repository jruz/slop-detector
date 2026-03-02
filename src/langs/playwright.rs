use std::fs;

use oxc::allocator::Allocator;
use oxc::ast::ast::Expression;
use oxc::parser::Parser;
use oxc::span::SourceType;
use oxc_ast_visit::{Visit, walk};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const PATTERNS: &[&str] = &["playwright/*.spec.ts"];
const EXCLUDES: &[&str] = &["fixtures"];

struct PlaywrightVisitor<'src> {
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
        page_errors: Vec::new(),
        page_request_errors: Vec::new(),
        request_errors: Vec::new(),
        path,
        source,
    }
}

impl<'src> Visit<'src> for PlaywrightVisitor<'src> {
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

    let mut rules: Vec<(&str, &Vec<String>, Option<&str>)> = Vec::new();

    if is_client {
        rules.push(("no-request", &visitor.request_errors, None));
        rules.push(("no-page-request", &visitor.page_request_errors, None));
    }

    if is_server {
        rules.push(("no-page", &visitor.page_errors, None));
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
                let parser = Parser::new(&allocator, &source, SourceType::ts());
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
                all_violations.extend(collect_violations(&visitor));
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
        errors,
        file_count,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        let path = format!("fixtures/playwright/{name}");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    fn parse_fixture_as(name: &str, virtual_path: &str) -> Vec<Violation> {
        let source = read_fixture(&format!("{name}.spec.ts"));
        let allocator = Allocator::default();
        let parser = Parser::new(&allocator, &source, SourceType::ts());
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
        assert_eq!(violations.len(), 1);
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
        assert_eq!(violations.len(), 1);
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
}
