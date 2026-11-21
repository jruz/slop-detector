use std::fs;

use lightningcss::stylesheet::{ParserOptions, StyleSheet};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[];
const PATTERNS: &[&str] = &["*.css"];

struct Rule {
    check: fn(&str, &StyleSheet<'_, '_>) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[Rule {
    check: check_no_stylelint_disable,
    hint: Some("fix the violation instead of disabling the rule"),
    name: "no-stylelint-disable",
}];

fn line_number(source: &str, byte_offset: usize) -> usize {
    source.get(..byte_offset).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

fn check_no_stylelint_disable(source: &str, _stylesheet: &StyleSheet<'_, '_>) -> Vec<String> {
    source
        .match_indices("stylelint-disable")
        .filter(|(offset, _)| match source.get(*offset..) {
            Some(rest) => {
                !rest.starts_with("stylelint-disable-")
                    || rest.starts_with("stylelint-disable-line")
                    || rest.starts_with("stylelint-disable-next-line")
            }
            None => false,
        })
        .map(|(offset, _)| {
            let line = line_number(source, offset);
            format!("line {line}: stylelint-disable")
        })
        .collect()
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

pub fn check() -> CheckResult {
    let (paths, mut errors) = find(PATTERNS, EXCLUDES);
    let mut violations = Vec::new();
    let mut file_count = 0;

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => {
                let parser_options = ParserOptions::default();
                match StyleSheet::parse(&source, parser_options) {
                    Ok(stylesheet) => {
                        file_count += 1;
                        for rule in RULES {
                            let rule_errors = (rule.check)(&source, &stylesheet);
                            if !rule_errors.is_empty() {
                                violations.push(Violation {
                                    errors: rule_errors,
                                    file: path.clone(),
                                    hint: rule.hint.map(String::from),
                                    rule: rule.name.to_string(),
                                });
                            }
                        }
                    }
                    Err(err) => {
                        errors.push(AppError::Parse {
                            file: path,
                            message: err.to_string(),
                        });
                    }
                }
            }
            Err(err) => {
                let error = crate::types::read_error(path, &err);
                errors.push(error);
            }
        }
    }

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

    fn parse_fixture(name: &str) -> (String, StyleSheet<'static, 'static>) {
        let path = format!("tests/fixtures/css/{name}.css");
        let source = fs::read_to_string(&path).expect("fixture should exist");
        let source_box = source.into_boxed_str();
        let source_leaked: &'static str = Box::leak(source_box);
        let parser_options = ParserOptions::default();
        let stylesheet = StyleSheet::parse(source_leaked, parser_options).expect("valid css");
        (source_leaked.to_string(), stylesheet)
    }

    #[test]
    fn no_stylelint_disable_detects_violation() {
        let (source, stylesheet) = parse_fixture("no-stylelint-disable");
        let errors = check_no_stylelint_disable(&source, &stylesheet);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_stylelint_disable_allows_valid() {
        let (source, stylesheet) = parse_fixture("valid-no-stylelint-disable");
        let errors = check_no_stylelint_disable(&source, &stylesheet);
        assert_eq!(errors, Vec::<String>::new());
    }
}
