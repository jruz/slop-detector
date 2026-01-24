use std::fs;

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
    check: fn(source: &str) -> Vec<String>,
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
}

fn line_number(source: &str, byte_offset: usize) -> usize {
    source.get(..byte_offset).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

const IGNORED_ERROR_PATTERN: &str = "Error(_)";

fn check_no_ignored_error(source: &str) -> Vec<String> {
    source
        .match_indices(IGNORED_ERROR_PATTERN)
        .map(|(pos, _)| {
            let line = line_number(source, pos);
            format!("line {line}: {IGNORED_ERROR_PATTERN}")
        })
        .collect()
}

const UNWRAP_PATTERN: &str = ".unwrap\x28";

fn check_no_unwrap(source: &str) -> Vec<String> {
    source
        .match_indices(UNWRAP_PATTERN)
        .map(|(pos, _)| {
            let line = line_number(source, pos);
            format!("line {line}: {UNWRAP_PATTERN})")
        })
        .collect()
}

struct ParsedGleamToml {
    path: String,
    toml: toml::Value,
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

fn parse_gleam(paths: Vec<String>) -> (Vec<ParsedGleam>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => parsed.push(ParsedGleam { path, source }),
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
    let (gleam_paths, mut errors) = find(GLEAM_PATTERNS, EXCLUDES);
    let (gleam_toml_paths, toml_glob_errors) = find(GLEAM_TOML_PATTERNS, EXCLUDES);
    errors.extend(toml_glob_errors);

    let (parsed_gleam, gleam_errors) = parse_gleam(gleam_paths);
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
        let source = read_fixture("no-ignored-error.gleam");
        let errors = check_no_ignored_error(&source);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_ignored_error_allows_valid() {
        let source = read_fixture("valid-no-ignored-error.gleam");
        let errors = check_no_ignored_error(&source);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_detects_violation() {
        let source = read_fixture("no-unwrap.gleam");
        let errors = check_no_unwrap(&source);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_allows_valid() {
        let source = read_fixture("valid-no-unwrap.gleam");
        let errors = check_no_unwrap(&source);
        assert!(errors.is_empty());
    }
}
