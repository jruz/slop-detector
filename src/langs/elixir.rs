use std::fs;

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["_build", "deps", "fixtures"];
const PATTERNS: &[&str] = &["*.ex", "*.exs"];

struct Rule {
    check: fn(&str) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[Rule {
    check: check_no_inline_style,
    hint: Some("use CSS classes instead of inline styles"),
    name: "no-inline-style",
}];

fn line_number(source: &str, byte_offset: usize) -> usize {
    source.get(..byte_offset).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

fn check_no_inline_style(source: &str) -> Vec<String> {
    source
        .match_indices("style=\"")
        .map(|(offset, _)| {
            let line = line_number(source, offset);
            format!("line {line}: style=")
        })
        .collect()
}

struct ParsedElixir {
    path: String,
    source: String,
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

fn parse(paths: Vec<String>) -> (Vec<ParsedElixir>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => parsed.push(ParsedElixir { path, source }),
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn run_rules(files: &[ParsedElixir], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.source);
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
    let (parsed, read_errors) = parse(paths);
    errors.extend(read_errors);
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
        let path = format!("fixtures/elixir/{name}.ex");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    #[test]
    fn no_inline_style_detects_violation() {
        let source = read_fixture("no-inline-style");
        let errors = check_no_inline_style(&source);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_inline_style_allows_valid() {
        let source = read_fixture("valid-no-inline-style");
        let errors = check_no_inline_style(&source);
        assert!(errors.is_empty());
    }
}
