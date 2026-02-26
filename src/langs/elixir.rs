use std::fs;

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["_build", "deps", "fixtures"];
const PATTERNS: &[&str] = &["*.ex", "*.exs"];

struct Rule {
    check: fn(&str) -> Vec<String>,
    filename: Option<&'static str>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_no_inline_style,
        filename: None,
        hint: Some("use CSS classes instead of inline styles"),
        name: "no-inline-style",
    },
    Rule {
        check: check_pinned_dep_versions,
        filename: Some("mix.exs"),
        hint: Some("pin to exact versions"),
        name: "pinned-dep-versions",
    },
    Rule {
        check: check_pinned_elixir_version,
        filename: Some("mix.exs"),
        hint: Some("pin to exact matching the flake"),
        name: "pinned-elixir-version",
    },
];

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

const UNPINNED_OPERATORS: &[&str] = &["~>", ">=", ">", "<=", "<"];

fn extract_version(line: &str) -> Option<&str> {
    let start = line.find('"')?;
    let rest = line.get(start + 1..)?;
    let end = rest.find('"')?;
    rest.get(..end)
}

fn is_unpinned_version(version: &str) -> bool {
    let trimmed = version.trim();
    UNPINNED_OPERATORS.iter().any(|op| trimmed.starts_with(op))
}

fn check_pinned_elixir_version(source: &str) -> Vec<String> {
    source
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            if !line.contains("elixir:") {
                return None;
            }
            let version = extract_version(line)?;
            if !is_unpinned_version(version) {
                return None;
            }
            Some(format!("line {}: \"{}\" not pinned", idx + 1, version))
        })
        .collect()
}

fn check_pinned_dep_versions(source: &str) -> Vec<String> {
    source
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            if !line.contains("{:") {
                return None;
            }
            let version = extract_version(line)?;
            if !is_unpinned_version(version) {
                return None;
            }
            Some(format!("line {}: \"{}\" not pinned", idx + 1, version))
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

fn matches_filename(path: &str, filename: &str) -> bool {
    path.ends_with(filename)
}

fn run_rules(files: &[ParsedElixir], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                if let Some(filename) = rule.filename
                    && !matches_filename(&file.path, filename)
                {
                    return None;
                }
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

    fn read_fixture(name: &str, ext: &str) -> String {
        let path = format!("fixtures/elixir/{name}.{ext}");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    #[test]
    fn no_inline_style_detects_violation() {
        let source = read_fixture("no-inline-style", "ex");
        let errors = check_no_inline_style(&source);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_inline_style_allows_valid() {
        let source = read_fixture("valid-no-inline-style", "ex");
        let errors = check_no_inline_style(&source);
        assert!(errors.is_empty());
    }

    #[test]
    fn pinned_dep_versions_detects_violation() {
        let source = read_fixture("pinned-versions", "exs");
        let errors = check_pinned_dep_versions(&source);
        assert_eq!(errors.len(), 5);
    }

    #[test]
    fn pinned_dep_versions_allows_valid() {
        let source = read_fixture("valid-pinned-versions", "exs");
        let errors = check_pinned_dep_versions(&source);
        assert!(errors.is_empty());
    }

    #[test]
    fn pinned_elixir_version_detects_violation() {
        let source = read_fixture("pinned-versions", "exs");
        let errors = check_pinned_elixir_version(&source);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn pinned_elixir_version_allows_valid() {
        let source = read_fixture("valid-pinned-versions", "exs");
        let errors = check_pinned_elixir_version(&source);
        assert!(errors.is_empty());
    }
}
